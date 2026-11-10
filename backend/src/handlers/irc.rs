//! The relay socket between the referee pages and Bancho's IRC server (GET /api/irc),
//! for the referee's own account. The connection itself, shared by all of a user's
//! tabs, is a session in crate::bancho::relay; this is one tab's end of it.

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
use tokio::{
    sync::broadcast::error::RecvError,
    time::{timeout, Duration},
};
use uuid::Uuid;

use super::check_origin;
use crate::{
    auth::{AuthUser, SessionKey},
    bancho::{
        irc::{clean_text, same_name, valid_name, LOGIN_TIMEOUT},
        relay::{Attached, CloseReason, Command, Outgoing, ServerMsg},
    },
    db,
    error::AppError,
    role::Role,
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().route("/irc", get(connect))
}

// What the page sends. The first message must be `Login` or `Resume`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    // Joins the user's Bancho connection if another tab has one, otherwise logs in.
    Login { username: String, password: String },
    // Joins the user's Bancho connection if another tab has one.
    Resume,
    Send { target: String, text: String },
    // Closes a private conversation in every tab and forgets its history.
    Close { name: String },
    // Ends the Bancho connection, for every tab.
    Quit,
    // The match this tab has open (null: none). Its room's bot makes the referee one of
    // its referees and the connection joins the room's channel; sent again whenever the
    // match's rooms change.
    Attend { match_id: Option<Uuid> },
}

// The most a tab's message may be: a login, or a chat line (IRC's are under 512 bytes).
const MAX_MESSAGE: usize = 4096;
// How often a tab checks its session again, besides whenever its user's roles change or
// they log out.
const RECHECK_EVERY: Duration = Duration::from_secs(5 * 60);

// GET /api/irc — referees and up. Upgrades to the relay WebSocket.
async fn connect(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    check_origin(&state, &headers)?;
    let user_id = user.id;
    let session = SessionKey::of(&jar);
    Ok(ws
        .max_message_size(MAX_MESSAGE)
        .on_upgrade(move |socket| tab(socket, state, user_id, session)))
}

// Whether the tab's user may still use the relay: signed in on the same session and
// still allowed Referee.
async fn still_referee(state: &AppState, session: &SessionKey) -> bool {
    match session.user(&state.db).await {
        Ok(user) => user.is_some_and(|user| user.roles.allows(Role::Referee)),
        Err(err) => {
            tracing::warn!(error = ?err, "could not check a relay socket's session");
            true
        }
    }
}

async fn send(socket: &mut WebSocket, msg: &ServerMsg) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => socket.send(Message::Text(json.into())).await.is_ok(),
        Err(_) => false,
    }
}

async fn close(mut socket: WebSocket, reason: CloseReason) {
    send(&mut socket, &ServerMsg::Closed { reason }).await;
    let _ = socket.send(Message::Close(None)).await;
}

// Reads the next JSON message from the page. None when the socket is closed.
async fn next_client_msg(socket: &mut WebSocket) -> Option<Result<ClientMsg, ()>> {
    loop {
        match socket.recv().await? {
            Ok(Message::Text(text)) => return Some(serde_json::from_str(&text).map_err(|_| ())),
            Ok(Message::Close(_)) | Err(_) => return None,
            Ok(_) => continue,
        }
    }
}

// One tab's WebSocket: joins or starts the user's session, then passes messages both
// ways until the tab or the session goes away.
async fn tab(mut socket: WebSocket, state: AppState, user_id: Uuid, session: SessionKey) {
    let attached = match timeout(LOGIN_TIMEOUT, next_client_msg(&mut socket)).await {
        Ok(Some(Ok(ClientMsg::Resume))) => match state.irc_users.attach(user_id) {
            Some(attached) => attached,
            None => {
                send(&mut socket, &ServerMsg::NoSession).await;
                return close(socket, CloseReason::NoSession).await;
            }
        },
        Ok(Some(Ok(ClientMsg::Login { username, password }))) => {
            // osu! usernames use underscores for spaces on IRC.
            let nick = username.trim().replace(' ', "_");
            if !valid_name(&nick) || password.is_empty() || password.chars().any(|c| c.is_control())
            {
                send(&mut socket, &ServerMsg::LoginFailed).await;
                return close(socket, CloseReason::InvalidLogin).await;
            }
            // Bancho keeps one IRC connection per account, so this one would replace the bot's.
            match db::bancho::list_bots(&state.db).await {
                Ok(bots) if bots.iter().any(|b| same_name(&b.username, &nick)) => {
                    send(&mut socket, &ServerMsg::BotAccount).await;
                    return close(socket, CloseReason::BotAccount).await;
                }
                Ok(_) => {}
                Err(err) => {
                    tracing::error!(error = ?err, "could not read the bot accounts");
                    return close(socket, CloseReason::AccountCheckFailed).await;
                }
            }
            match state.irc_users.login(&state, user_id, nick, password) {
                Ok(attached) => attached,
                Err(wait) => {
                    let retry_after = wait.as_secs().max(1);
                    send(&mut socket, &ServerMsg::LoginBlocked { retry_after }).await;
                    return close(socket, CloseReason::TooManyFailedLogins).await;
                }
            }
        }
        _ => return close(socket, CloseReason::LoginExpected).await,
    };
    let id = rand::random();
    pump(socket, attached, &state, id, user_id, &session).await;
    state.bancho.tab_closed(id);
}

async fn pump(
    mut socket: WebSocket,
    mut tab: Attached,
    state: &AppState,
    id: u64,
    user_id: Uuid,
    session: &SessionKey,
) {
    for msg in &tab.backlog {
        if !send(&mut socket, msg).await {
            return;
        }
    }
    let mut users = state.live.user_changes();
    let mut recheck = tokio::time::interval(RECHECK_EVERY);
    recheck.tick().await;
    loop {
        tokio::select! {
            changed = users.recv() => {
                let ours = match changed {
                    Ok(changed) => changed == user_id,
                    Err(RecvError::Lagged(_)) => true,
                    Err(RecvError::Closed) => false,
                };
                if ours && !still_referee(state, session).await {
                    return close(socket, CloseReason::NoLongerReferee).await;
                }
            }
            _ = recheck.tick() => {
                if !still_referee(state, session).await {
                    return close(socket, CloseReason::NoLongerReferee).await;
                }
            }
            event = tab.events.recv() => match event {
                Ok(json) => {
                    if socket.send(Message::Text(json.as_str().into())).await.is_err() {
                        return;
                    }
                }
                Err(RecvError::Lagged(_)) => return close(socket, CloseReason::FellBehind).await,
                // The session ended and its Closed message went out before this.
                Err(RecvError::Closed) => break,
            },
            msg = next_client_msg(&mut socket) => {
                let Some(msg) = msg else { return };
                let command = match msg {
                    Ok(ClientMsg::Send { target, text }) => {
                        let text = clean_text(&text);
                        (valid_name(&target) && !text.trim().is_empty())
                            .then_some(Command::Send(Outgoing::Privmsg { target, text }))
                    }
                    Ok(ClientMsg::Close { name }) if valid_name(&name) && !name.starts_with('#') => {
                        Some(Command::Close(name))
                    }
                    Ok(ClientMsg::Quit) => Some(Command::Quit),
                    Ok(ClientMsg::Attend { match_id }) => {
                        state.bancho.attend(id, tab.session.link(), match_id);
                        None
                    }
                    _ => None,
                };
                if let Some(command) = command {
                    let _ = tab.commands.send(command);
                }
            }
        }
    }
    let _ = socket.send(Message::Close(None)).await;
}
