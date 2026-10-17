//! Pushes changes to the open pages as they happen, over one WebSocket per tab
//! (GET /api/live), so a score saved on the referee page shows everywhere at once.
//!
//! Everyone gets `score` (a bracket match's score changed, with the new figures) and
//! `bracket` (something else about the bracket changed: reload it). Staff who can
//! open the bracket admin page also get who has each match's referee page open: a
//! tab tells the server which match's referee page it shows (`viewing`), and every
//! staff tab hears the new list for that match (`presence`), plus all of them when it
//! connects (`presence_all`). Nothing here is stored; it's what the open tabs say.
//!
//! The public match page also hears `match` (something about one match changed that
//! the bracket doesn't show, like its bans: reload that match) and, for the match it
//! says it shows (`watch`), every line said in its rooms as the relay saves it (`chat`).

use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::{header, HeaderMap},
    response::IntoResponse,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::sync::broadcast::{self, error::RecvError};
use uuid::Uuid;

use crate::{auth::AuthUser, db, db::chat::ChatMessage, error::AppError, role::Role, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/live", get(connect))
}

// How many events a slow tab may fall behind before it's told to reload instead.
const BUFFER: usize = 256;
// Keeps idle connections open through proxies that close quiet sockets.
const PING_EVERY: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Serialize)]
pub struct Viewer {
    id: Uuid,
    username: String,
    avatar_url: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg<'a> {
    Score {
        match_id: Uuid,
        team1_score: Option<i32>,
        team2_score: Option<i32>,
    },
    Bracket,
    Match {
        match_id: Uuid,
    },
    Chat {
        mp_id: i64,
        message: &'a ChatMessage,
    },
    Presence {
        match_id: Uuid,
        viewers: &'a [Viewer],
    },
    PresenceAll {
        matches: HashMap<Uuid, Vec<Viewer>>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    // The match whose referee page this tab shows, or null when it left it.
    Viewing { match_id: Option<Uuid> },
    // The match whose public page this tab shows, or null: its rooms' chat is sent.
    // Sent again when the match's rooms change.
    Watch { match_id: Option<Uuid> },
}

// One event, serialized once for every tab.
#[derive(Debug)]
struct Event {
    json: String,
    staff_only: bool,
    // Only for the tabs watching this room's match.
    room: Option<i64>,
}

// Who has each match's referee page open: by match, each tab with its user, in the
// order they opened it.
#[derive(Default)]
struct Presence {
    tabs: HashMap<Uuid, Vec<(u64, Viewer)>>,
}

impl Presence {
    // The match's viewers, each user once, in the order they first opened it.
    fn viewers(&self, match_id: Uuid) -> Vec<Viewer> {
        let mut out: Vec<Viewer> = Vec::new();
        for (_, viewer) in self.tabs.get(&match_id).into_iter().flatten() {
            if !out.iter().any(|v| v.id == viewer.id) {
                out.push(viewer.clone());
            }
        }
        out
    }

    fn all(&self) -> HashMap<Uuid, Vec<Viewer>> {
        self.tabs.keys().map(|&id| (id, self.viewers(id))).collect()
    }

    // Moves a tab to another match (or none). Returns the matches whose lists changed.
    fn set(&mut self, tab: u64, viewer: &Viewer, match_id: Option<Uuid>) -> Vec<Uuid> {
        let mut changed = Vec::new();
        self.tabs.retain(|&id, tabs| {
            let before = tabs.len();
            tabs.retain(|(t, _)| *t != tab);
            if tabs.len() != before && Some(id) != match_id {
                changed.push(id);
            }
            !tabs.is_empty()
        });
        if let Some(id) = match_id {
            self.tabs.entry(id).or_default().push((tab, viewer.clone()));
            changed.push(id);
        }
        changed
    }
}

struct Inner {
    events: broadcast::Sender<Arc<Event>>,
    presence: Mutex<Presence>,
    next_tab: AtomicU64,
}

// The hub every tab's socket listens to. Cheap to clone.
#[derive(Clone)]
pub struct Live(Arc<Inner>);

impl Default for Live {
    fn default() -> Self {
        Self(Arc::new(Inner {
            events: broadcast::channel(BUFFER).0,
            presence: Mutex::default(),
            next_tab: AtomicU64::new(1),
        }))
    }
}

impl Live {
    fn emit(&self, msg: &ServerMsg, staff_only: bool) {
        self.send(msg, staff_only, None);
    }

    fn send(&self, msg: &ServerMsg, staff_only: bool, room: Option<i64>) {
        let Ok(json) = serde_json::to_string(msg) else {
            return;
        };
        // An error only means no tab is listening.
        let _ = self.0.events.send(Arc::new(Event {
            json,
            staff_only,
            room,
        }));
    }

    // A bracket match's score was saved. Call after the change is committed.
    pub fn score(&self, match_id: Uuid, team1_score: Option<i32>, team2_score: Option<i32>) {
        self.emit(
            &ServerMsg::Score {
                match_id,
                team1_score,
                team2_score,
            },
            false,
        );
    }

    // Anything else on the bracket changed (teams, times, rooms, staff, conclusions),
    // so pages reload it. Call after the change is committed.
    pub fn bracket_changed(&self) {
        self.emit(&ServerMsg::Bracket, false);
    }

    // Something about one match changed that the bracket doesn't show (bans, picks,
    // hand-awarded results, a lobby's teams), so its match page reloads it. Call after
    // the change is committed.
    pub fn match_changed(&self, match_id: Uuid) {
        self.emit(&ServerMsg::Match { match_id }, false);
    }

    // A line said in a multiplayer room, once it's saved; only tabs watching the match
    // the room belongs to get it.
    pub fn chat(&self, mp_id: i64, message: &ChatMessage) {
        self.send(&ServerMsg::Chat { mp_id, message }, false, Some(mp_id));
    }

    fn set_viewing(&self, tab: u64, viewer: &Viewer, match_id: Option<Uuid>) {
        let mut presence = self.0.presence.lock().unwrap();
        for id in presence.set(tab, viewer, match_id) {
            let viewers = presence.viewers(id);
            self.emit(
                &ServerMsg::Presence {
                    match_id: id,
                    viewers: &viewers,
                },
                true,
            );
        }
    }
}

// Staff who can open the bracket admin page see who's on which referee page.
fn sees_presence(user: &AuthUser) -> bool {
    [Role::Referee, Role::Streamer, Role::MapPooler]
        .into_iter()
        .any(|role| user.roles.allows(role))
}

// GET /api/live — anyone, signed in or not. Upgrades to the live WebSocket.
pub async fn connect(
    State(state): State<AppState>,
    user: Option<AuthUser>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    // CORS doesn't cover WebSockets, so check that the page is ours.
    if let Some(origin) = headers.get(header::ORIGIN) {
        let ours = state.config.frontend_url.trim_end_matches('/');
        if origin.as_bytes() != ours.as_bytes() {
            return Err(AppError::Forbidden);
        }
    }
    let staff = user.as_ref().is_some_and(sees_presence);
    // Only those who can open a referee page can say they're on one.
    let viewer = match &user {
        Some(user) if user.roles.allows(Role::Referee) => db::user::find_user(&state.db, user.id)
            .await?
            .map(|profile| Viewer {
                id: profile.osu.id,
                username: profile.osu.username,
                avatar_url: profile.osu.avatar_url,
            }),
        _ => None,
    };
    let live = state.live.clone();
    let db = state.db.clone();
    Ok(ws.on_upgrade(move |socket| tab(socket, live, db, staff, viewer)))
}

async fn send(socket: &mut WebSocket, json: String) -> bool {
    socket.send(Message::Text(json)).await.is_ok()
}

// The rooms of the match a tab watches; none for a match that doesn't exist.
async fn watched_rooms(db: &PgPool, match_id: Option<Uuid>) -> HashSet<i64> {
    let Some(match_id) = match_id else {
        return HashSet::new();
    };
    match db::matches::find_match(db, match_id).await {
        Ok(m) => m
            .map(|m| m.mp_ids.into_iter().collect())
            .unwrap_or_default(),
        Err(err) => {
            tracing::error!(error = ?err, %match_id, "could not read a watched match's rooms");
            HashSet::new()
        }
    }
}

async fn tab(mut socket: WebSocket, live: Live, db: PgPool, staff: bool, viewer: Option<Viewer>) {
    let id = live.0.next_tab.fetch_add(1, Ordering::Relaxed);
    // The rooms whose chat this tab is sent.
    let mut watching: HashSet<i64> = HashSet::new();
    let mut events = live.0.events.subscribe();
    if staff {
        let matches = live.0.presence.lock().unwrap().all();
        let Ok(json) = serde_json::to_string(&ServerMsg::PresenceAll { matches }) else {
            return;
        };
        if !send(&mut socket, json).await {
            return;
        }
    }
    let mut ping = tokio::time::interval(PING_EVERY);
    ping.tick().await;

    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    if event.staff_only && !staff {
                        continue;
                    }
                    if event.room.is_some_and(|room| !watching.contains(&room)) {
                        continue;
                    }
                    if !send(&mut socket, event.json.clone()).await {
                        break;
                    }
                }
                // Missed some: a reload brings the page back in step. Presence lists
                // are sent whole, so the next change of each fixes them.
                Err(RecvError::Lagged(_)) => {
                    let Ok(json) = serde_json::to_string(&ServerMsg::Bracket) else { break };
                    if !send(&mut socket, json).await {
                        break;
                    }
                }
                Err(RecvError::Closed) => break,
            },
            msg = socket.recv() => match msg {
                Some(Ok(Message::Text(text))) => match serde_json::from_str::<ClientMsg>(&text) {
                    Ok(ClientMsg::Viewing { match_id }) => {
                        if let Some(viewer) = &viewer {
                            live.set_viewing(id, viewer, match_id);
                        }
                    }
                    Ok(ClientMsg::Watch { match_id }) => {
                        watching = watched_rooms(&db, match_id).await;
                    }
                    Err(_) => {}
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            _ = ping.tick() => {
                if socket.send(Message::Ping(Vec::new())).await.is_err() {
                    break;
                }
            }
        }
    }
    if let Some(viewer) = &viewer {
        live.set_viewing(id, viewer, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn viewer(n: u128) -> Viewer {
        Viewer {
            id: Uuid::from_u128(n),
            username: format!("user{n}"),
            avatar_url: String::new(),
        }
    }

    #[test]
    fn lists_each_user_once() {
        let mut p = Presence::default();
        let m = Uuid::from_u128(100);
        assert_eq!(p.set(1, &viewer(1), Some(m)), vec![m]);
        p.set(2, &viewer(1), Some(m));
        p.set(3, &viewer(2), Some(m));
        let names: Vec<_> = p.viewers(m).into_iter().map(|v| v.username).collect();
        assert_eq!(names, ["user1", "user2"]);
    }

    #[test]
    fn moving_and_leaving_update_both_matches() {
        let mut p = Presence::default();
        let (a, b) = (Uuid::from_u128(100), Uuid::from_u128(200));
        p.set(1, &viewer(1), Some(a));
        let mut changed = p.set(1, &viewer(1), Some(b));
        changed.sort();
        assert_eq!(changed, vec![a, b]);
        assert!(p.viewers(a).is_empty());
        assert_eq!(p.set(1, &viewer(1), None), vec![b]);
        assert!(p.all().is_empty());
    }
}
