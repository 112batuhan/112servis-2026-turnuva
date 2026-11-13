//! The live socket: one WebSocket per tab (GET /api/live) to the hub in crate::live,
//! which says what each event is. A tab tells the server which match's referee page
//! it shows (`viewing`) and which match it follows (`watch`); the server sends it the
//! events that reach it.

use std::{collections::HashSet, time::Duration};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    http::HeaderMap,
    response::IntoResponse,
    routing::get,
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Deserialize;
use sqlx::PgPool;
use tokio::{
    sync::broadcast::error::RecvError,
    time::{sleep_until, Instant},
};
use uuid::Uuid;

use super::check_origin;
use crate::{
    auth::{AuthUser, SessionKey},
    db,
    error::AppError,
    live::{Live, TabFilter, Viewer},
    role::Role,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/live", get(connect))
}

// Keeps idle connections open through proxies that close quiet sockets.
const PING_EVERY: Duration = Duration::from_secs(30);
// How often a signed-in tab checks its session again, besides whenever its user's roles
// change or they log out, so an expired session doesn't keep staff events.
const RECHECK_EVERY: Duration = Duration::from_secs(5 * 60);
// The most a tab's message may be; theirs are a few dozen bytes.
const MAX_MESSAGE: usize = 4096;
// A tab's `watch`es are applied at most this often (each reads the match's rooms); a
// newer one waiting replaces the one before it.
const WATCH_EVERY: Duration = Duration::from_millis(500);

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
async fn connect(
    State(state): State<AppState>,
    user: Option<AuthUser>,
    jar: CookieJar,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    check_origin(&state, &headers)?;
    let access = Access::of(user.as_ref());
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
    let session = user.map(|user| (user.id, SessionKey::of(&jar)));
    Ok(ws
        .max_message_size(MAX_MESSAGE)
        .on_upgrade(move |socket| tab(socket, live, db, access, session, viewer)))
}

// What a tab is sent, by who's signed in on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Access {
    // Staff events and presence.
    staff: bool,
    // Only those who can open a referee page can say they're on one.
    referee: bool,
}

impl Access {
    fn of(user: Option<&AuthUser>) -> Self {
        Self {
            staff: user.is_some_and(sees_presence),
            referee: user.is_some_and(|user| user.roles.allows(Role::Referee)),
        }
    }
}

async fn send(socket: &mut WebSocket, json: String) -> bool {
    socket.send(Message::Text(json.into())).await.is_ok()
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

// Whether a signed-in tab may go on as it is: its session is still there and its user
// holds what they held when it connected. A tab that may not is closed, and the page
// connects again with what they hold now.
async fn still_allowed(db: &PgPool, session: &SessionKey, access: Access) -> bool {
    match session.user(db).await {
        Ok(user) => user.is_some() && Access::of(user.as_ref()) == access,
        // Can't tell: keep it until the next check.
        Err(err) => {
            tracing::warn!(error = ?err, "could not check a live socket's session");
            true
        }
    }
}

async fn tab(
    mut socket: WebSocket,
    live: Live,
    db: PgPool,
    access: Access,
    session: Option<(Uuid, SessionKey)>,
    viewer: Option<Viewer>,
) {
    let Access { staff, referee } = access;
    let id = live.next_tab();
    let mut filter = TabFilter {
        staff,
        user: session.as_ref().map(|(user_id, _)| *user_id),
        ..TabFilter::default()
    };
    let mut events = live.events();
    if staff {
        let Some(json) = live.presence_json() else {
            return;
        };
        if !send(&mut socket, json).await {
            return;
        }
    }
    let mut ping = tokio::time::interval(PING_EVERY);
    ping.tick().await;
    let mut recheck = tokio::time::interval(RECHECK_EVERY);
    recheck.tick().await;
    let mut users = live.user_changes();
    // The latest `watch` not applied yet, and when the next may be.
    let mut waiting: Option<(Option<Uuid>, bool)> = None;
    let mut next_watch = Instant::now();

    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    if !event.reaches(&filter) {
                        continue;
                    }
                    if !send(&mut socket, event.json().to_owned()).await {
                        break;
                    }
                }
                // Missed some: a reload brings the page back in step. Presence lists
                // are sent whole, so the next change of each fixes them.
                Err(RecvError::Lagged(_)) => {
                    let Some(json) = Live::reload_json() else { break };
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
                        waiting = Some((match_id, spectate));
                    }
                    Err(_) => {}
                },
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                Some(Ok(_)) => {}
            },
            () = sleep_until(next_watch), if waiting.is_some() => {
                let Some((match_id, spectate)) = waiting.take() else { continue };
                next_watch = Instant::now() + WATCH_EVERY;
                filter.rooms = watched_rooms(&db, match_id).await;
                filter.watched = match_id;
                // Only a match that has a room is watched; a made-up id isn't counted.
                let now = match_id.filter(|_| spectate && !filter.rooms.is_empty());
                if now != filter.spectating {
                    filter.spectating = now;
                    live.set_spectating(id, viewer.as_ref(), now);
                }
            }
            changed = users.recv(), if session.is_some() => {
                let Some((user_id, key)) = &session else { continue };
                let ours = match changed {
                    Ok(changed) => changed == *user_id,
                    Err(RecvError::Lagged(_)) => true,
                    Err(RecvError::Closed) => false,
                };
                if ours && !still_allowed(&db, key, access).await {
                    break;
                }
            }
            _ = recheck.tick(), if session.is_some() => {
                let Some((_, key)) = &session else { continue };
                if !still_allowed(&db, key, access).await {
                    break;
                }
            }
            _ = ping.tick() => {
                if socket.send(Message::Ping(Default::default())).await.is_err() {
                    break;
                }
            }
        }
    }
    if let (true, Some(viewer)) = (referee, &viewer) {
        live.set_viewing(id, viewer, None);
    }
    if filter.spectating.is_some() {
        live.set_spectating(id, viewer.as_ref(), None);
    }
}
