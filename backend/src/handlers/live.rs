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
//! Staff also hear `referee`: something only the referee page shows changed about a
//! match (its room options, the maps left out of the score, where its results are
//! read from), so the referee pages open on it reload it.
//!
//! The public match page also hears `match` (something about one match changed that
//! the bracket doesn't show, like its bans: reload that match) and, for the match it
//! says it shows (`watch`), every line said in its rooms as the room's bot saves it
//! (`chat`), the room itself as the bots read it whenever it changes (`room`, the same
//! view on every page) and who else is watching it (`spectators`: the signed-in ones
//! by name, each user once, and a count of the tabs nobody is signed in on). Staff
//! also hear `bancho` when a bot connects, drops or takes a room, for the bots page.
//!
//! The room manager (crate::bancho) listens too, through `changes`: whatever the pages
//! are told changed about a match, it reads again.

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

use crate::{
    auth::AuthUser, bancho::view::LiveRoom, db, db::chat::ChatMessage, error::AppError, role::Role,
    AppState,
};

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
    Referee {
        match_id: Uuid,
    },
    Chat {
        mp_id: i64,
        message: &'a ChatMessage,
    },
    Room {
        match_id: Uuid,
        room: &'a LiveRoom,
    },
    Bancho,
    Presence {
        match_id: Uuid,
        viewers: &'a [Viewer],
    },
    Spectators {
        match_id: Uuid,
        viewers: &'a [Viewer],
        guests: usize,
    },
    PresenceAll {
        matches: HashMap<Uuid, Vec<Viewer>>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    // The match whose referee page this tab shows, or null when it left it.
    Viewing {
        match_id: Option<Uuid>,
    },
    // The match whose rooms' chat this tab wants, or null. Sent again when the match's
    // rooms change. A public match page also counts among its spectators; a referee
    // page that isn't in the room on Bancho reads the chat this way without being one.
    Watch {
        match_id: Option<Uuid>,
        #[serde(default = "spectate_by_default")]
        spectate: bool,
    },
}

// One event, serialized once for every tab.
#[derive(Debug)]
struct Event {
    json: String,
    staff_only: bool,
    // Only for the tabs watching this room's match.
    room: Option<i64>,
    // Only for the tabs counted among this match's spectators.
    watchers_of: Option<Uuid>,
    // Only for the tabs watching this match, spectating or not.
    watching: Option<Uuid>,
}

// What the room manager hears: a match changed (read it again), or something that may
// touch any of them did.
#[derive(Debug, Clone, Copy)]
pub enum Change {
    Match(Uuid),
    All,
}

// Who has a page of each match open (its referee page, or its public page): by match,
// each tab with its user (none for a tab nobody is signed in on), in the order they
// opened it.
#[derive(Default)]
struct Presence {
    tabs: HashMap<Uuid, Vec<(u64, Option<Viewer>)>>,
}

impl Presence {
    // The match's signed-in viewers, each user once, in the order they first opened it.
    fn viewers(&self, match_id: Uuid) -> Vec<Viewer> {
        let mut out: Vec<Viewer> = Vec::new();
        for viewer in self
            .tabs
            .get(&match_id)
            .into_iter()
            .flatten()
            .filter_map(|(_, v)| v.as_ref())
        {
            if !out.iter().any(|v| v.id == viewer.id) {
                out.push(viewer.clone());
            }
        }
        out
    }

    // The match's tabs nobody is signed in on.
    fn guests(&self, match_id: Uuid) -> usize {
        self.tabs
            .get(&match_id)
            .map_or(0, |tabs| tabs.iter().filter(|(_, v)| v.is_none()).count())
    }

    fn all(&self) -> HashMap<Uuid, Vec<Viewer>> {
        self.tabs.keys().map(|&id| (id, self.viewers(id))).collect()
    }

    // Moves a tab to another match (or none). Returns the matches whose lists changed.
    fn set(&mut self, tab: u64, viewer: Option<&Viewer>, match_id: Option<Uuid>) -> Vec<Uuid> {
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
            self.tabs
                .entry(id)
                .or_default()
                .push((tab, viewer.cloned()));
            changed.push(id);
        }
        changed
    }
}

struct Inner {
    events: broadcast::Sender<Arc<Event>>,
    changes: broadcast::Sender<Change>,
    // Referee pages.
    presence: Mutex<Presence>,
    // Public match pages.
    spectators: Mutex<Presence>,
    next_tab: AtomicU64,
}

// The hub every tab's socket listens to. Cheap to clone.
#[derive(Clone)]
pub struct Live(Arc<Inner>);

impl Default for Live {
    fn default() -> Self {
        Self(Arc::new(Inner {
            events: broadcast::channel(BUFFER).0,
            changes: broadcast::channel(BUFFER).0,
            presence: Mutex::default(),
            spectators: Mutex::default(),
            next_tab: AtomicU64::new(1),
        }))
    }
}

impl Live {
    fn emit(&self, msg: &ServerMsg, staff_only: bool) {
        self.send(msg, staff_only, None, None, None);
    }

    fn send(
        &self,
        msg: &ServerMsg,
        staff_only: bool,
        room: Option<i64>,
        watchers_of: Option<Uuid>,
        watching: Option<Uuid>,
    ) {
        let Ok(json) = serde_json::to_string(msg) else {
            return;
        };
        // An error only means no tab is listening.
        let _ = self.0.events.send(Arc::new(Event {
            json,
            staff_only,
            room,
            watchers_of,
            watching,
        }));
    }

    fn changed(&self, change: Change) {
        let _ = self.0.changes.send(change);
    }

    // Every change the pages hear about, for the room manager.
    pub fn changes(&self) -> broadcast::Receiver<Change> {
        self.0.changes.subscribe()
    }

    // A bracket match's score was saved. Call after the change is committed.
    pub fn score(&self, match_id: Uuid, team1_score: Option<i32>, team2_score: Option<i32>) {
        self.changed(Change::Match(match_id));
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
        self.changed(Change::All);
        self.emit(&ServerMsg::Bracket, false);
    }

    // Something about one match changed that the bracket doesn't show (bans, picks,
    // hand-awarded results, a lobby's teams), so its match page reloads it. Call after
    // the change is committed.
    pub fn match_changed(&self, match_id: Uuid) {
        self.changed(Change::Match(match_id));
        self.emit(&ServerMsg::Match { match_id }, false);
    }

    // Something only the referee page shows changed about a match (its room options,
    // "Sayma", where its results are read from); staff only. Call after the change is
    // committed.
    pub fn referee_changed(&self, match_id: Uuid) {
        self.changed(Change::Match(match_id));
        self.emit(&ServerMsg::Referee { match_id }, true);
    }

    // A line said in a multiplayer room, once it's saved; only tabs watching the match
    // the room belongs to get it.
    pub fn chat(&self, mp_id: i64, message: &ChatMessage) {
        self.send(
            &ServerMsg::Chat { mp_id, message },
            false,
            Some(mp_id),
            None,
            None,
        );
    }

    // The match's room as the bots read it now; only tabs watching the match get it.
    pub fn room(&self, match_id: Uuid, room: &LiveRoom) {
        self.send(
            &ServerMsg::Room { match_id, room },
            false,
            None,
            None,
            Some(match_id),
        );
    }

    // A bot connected, dropped, or took or left a room; staff only.
    pub fn bancho_changed(&self) {
        self.emit(&ServerMsg::Bancho, true);
    }

    fn set_viewing(&self, tab: u64, viewer: &Viewer, match_id: Option<Uuid>) {
        let mut presence = self.0.presence.lock().unwrap();
        for id in presence.set(tab, Some(viewer), match_id) {
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

    // Moves a tab to another match's public page (or none); the tabs on each match whose
    // list changed hear the new one.
    fn set_spectating(&self, tab: u64, viewer: Option<&Viewer>, match_id: Option<Uuid>) {
        let mut spectators = self.0.spectators.lock().unwrap();
        for id in spectators.set(tab, viewer, match_id) {
            let viewers = spectators.viewers(id);
            self.send(
                &ServerMsg::Spectators {
                    match_id: id,
                    viewers: &viewers,
                    guests: spectators.guests(id),
                },
                false,
                None,
                Some(id),
                None,
            );
        }
    }
}

fn spectate_by_default() -> bool {
    true
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
    let referee = user
        .as_ref()
        .is_some_and(|user| user.roles.allows(Role::Referee));
    // Signed in means signed in with osu!, so the name spectators see is the osu! one.
    let viewer = match &user {
        Some(user) => db::user::find_user(&state.db, user.id)
            .await?
            .map(|profile| Viewer {
                id: profile.osu.id,
                username: profile.osu.username,
                avatar_url: profile.osu.avatar_url,
            }),
        None => None,
    };
    let live = state.live.clone();
    let db = state.db.clone();
    Ok(ws.on_upgrade(move |socket| tab(socket, live, db, staff, referee, viewer)))
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

async fn tab(
    mut socket: WebSocket,
    live: Live,
    db: PgPool,
    staff: bool,
    referee: bool,
    viewer: Option<Viewer>,
) {
    let id = live.0.next_tab.fetch_add(1, Ordering::Relaxed);
    // The rooms whose chat this tab is sent, and their match.
    let mut watching: HashSet<i64> = HashSet::new();
    let mut watched: Option<Uuid> = None;
    // The match whose public page this tab shows, counted among its spectators.
    let mut spectating: Option<Uuid> = None;
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
                    if event.watchers_of.is_some_and(|m| spectating != Some(m)) {
                        continue;
                    }
                    if event.watching.is_some_and(|m| watched != Some(m)) {
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
                        if let (true, Some(viewer)) = (referee, &viewer) {
                            live.set_viewing(id, viewer, match_id);
                        }
                    }
                    Ok(ClientMsg::Watch { match_id, spectate }) => {
                        watching = watched_rooms(&db, match_id).await;
                        watched = match_id;
                        // Only a match that has a room is watched; a made-up id isn't counted.
                        let now = match_id.filter(|_| spectate && !watching.is_empty());
                        if now != spectating {
                            spectating = now;
                            live.set_spectating(id, viewer.as_ref(), now);
                        }
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
    if let (true, Some(viewer)) = (referee, &viewer) {
        live.set_viewing(id, viewer, None);
    }
    if spectating.is_some() {
        live.set_spectating(id, viewer.as_ref(), None);
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
        assert_eq!(p.set(1, Some(&viewer(1)), Some(m)), vec![m]);
        p.set(2, Some(&viewer(1)), Some(m));
        p.set(3, Some(&viewer(2)), Some(m));
        let names: Vec<_> = p.viewers(m).into_iter().map(|v| v.username).collect();
        assert_eq!(names, ["user1", "user2"]);
    }

    #[test]
    fn moving_and_leaving_update_both_matches() {
        let mut p = Presence::default();
        let (a, b) = (Uuid::from_u128(100), Uuid::from_u128(200));
        p.set(1, Some(&viewer(1)), Some(a));
        let mut changed = p.set(1, Some(&viewer(1)), Some(b));
        changed.sort();
        assert_eq!(changed, vec![a, b]);
        assert!(p.viewers(a).is_empty());
        assert_eq!(p.set(1, Some(&viewer(1)), None), vec![b]);
        assert!(p.all().is_empty());
    }

    #[test]
    fn counts_guest_tabs_apart_from_names() {
        let mut p = Presence::default();
        let m = Uuid::from_u128(100);
        p.set(1, None, Some(m));
        p.set(2, None, Some(m));
        p.set(3, Some(&viewer(1)), Some(m));
        assert_eq!(p.guests(m), 2);
        assert_eq!(p.viewers(m).len(), 1);
        p.set(1, None, None);
        assert_eq!(p.guests(m), 1);
    }
}
