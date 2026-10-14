//! A thin relay between the referee page and Bancho's IRC server. Browsers can't open
//! raw TCP connections, so the page talks JSON over a WebSocket and this handler holds
//! the IRC connection for it. It logs in with the referee's own IRC credentials, which
//! live only in memory for the life of the socket. It answers PINGs, paces outgoing
//! lines and saves what's said in multiplayer rooms (see RoomLog); everything Bancho
//! says goes to the page, which does all the match logic.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
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
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpStream},
    time::{sleep_until, timeout, Instant},
};

use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth::AuthUser, db, error::AppError, role::Role, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/irc", get(connect))
}

// Bancho mutes accounts that send too fast, so at most SEND_LIMIT lines go out in any
// SEND_WINDOW. The rest wait their turn in order.
const SEND_LIMIT: usize = 8;
const SEND_WINDOW: Duration = Duration::from_secs(5);
// How long the page has to send its login, and Bancho to accept it.
const LOGIN_TIMEOUT: Duration = Duration::from_secs(20);
// Leaves room for the prefix Bancho adds within IRC's 512-byte line limit.
const TEXT_MAX: usize = 400;
// A line longer than this is not IRC; drop the connection rather than buffer it.
const LINE_MAX: usize = 8192;

// What the page sends. The first message must be `Login`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    Login { username: String, password: String },
    Send { target: String, text: String },
    Join { channel: String },
    Part { channel: String },
}

// What the page receives.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg {
    // Bancho accepted the login.
    Connected {
        nick: String,
    },
    // Bancho rejected the username or IRC password.
    LoginFailed,
    // A chat line, including the ones we send (IRC doesn't echo those back).
    Message {
        from: String,
        target: String,
        text: String,
        action: bool,
    },
    Joined {
        channel: String,
    },
    Parted {
        channel: String,
    },
    // A command Bancho refused, e.g. joining a channel that doesn't exist.
    Error {
        message: String,
    },
    // The IRC connection is gone; the socket closes after this.
    Closed {
        reason: String,
    },
}

// GET /api/irc — referees and up. Upgrades to the relay WebSocket.
pub async fn connect(
    State(state): State<AppState>,
    user: AuthUser,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Referee)?;
    // CORS doesn't cover WebSockets, so check that the page is ours.
    if let Some(origin) = headers.get(header::ORIGIN) {
        let ours = state.config.frontend_url.trim_end_matches('/');
        if origin.as_bytes() != ours.as_bytes() {
            return Err(AppError::Forbidden);
        }
    }
    let addr = state.config.bancho_irc_addr.clone();
    let log = RoomLog {
        db: state.db.clone(),
        loggers: state.chat_loggers.clone(),
        conn: rand::random(),
        user_id: user.id,
    };
    Ok(ws.on_upgrade(move |socket| relay(socket, addr, log)))
}

async fn send(socket: &mut WebSocket, msg: &ServerMsg) -> bool {
    match serde_json::to_string(msg) {
        Ok(json) => socket.send(Message::Text(json)).await.is_ok(),
        Err(_) => false,
    }
}

async fn close(mut socket: WebSocket, reason: &str) {
    send(
        &mut socket,
        &ServerMsg::Closed {
            reason: reason.to_string(),
        },
    )
    .await;
    let _ = socket.close().await;
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

// IRC names and channels: no spaces, control characters or a leading colon, which
// would let them smuggle extra parameters or commands into a line.
fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with(':')
        && !s.chars().any(|c| c.is_whitespace() || c.is_control())
}

// Strips line breaks and cuts the text to TEXT_MAX bytes on a character boundary.
fn clean_text(text: &str) -> String {
    let mut out = String::new();
    for c in text
        .chars()
        .map(|c| if c == '\r' || c == '\n' { ' ' } else { c })
    {
        if out.len() + c.len_utf8() > TEXT_MAX {
            break;
        }
        out.push(c);
    }
    out
}

async fn relay(mut socket: WebSocket, addr: String, log: RoomLog) {
    let Ok(Some(Ok(ClientMsg::Login { username, password }))) =
        timeout(LOGIN_TIMEOUT, next_client_msg(&mut socket)).await
    else {
        return close(socket, "login expected").await;
    };
    // osu! usernames use underscores for spaces on IRC.
    let nick = username.trim().replace(' ', "_");
    if !valid_name(&nick) || password.is_empty() || password.chars().any(|c| c.is_control()) {
        send(&mut socket, &ServerMsg::LoginFailed).await;
        return close(socket, "invalid login").await;
    }

    let stream = match timeout(LOGIN_TIMEOUT, TcpStream::connect(&addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "could not reach Bancho");
            return close(socket, "could not reach Bancho").await;
        }
        Err(_) => return close(socket, "could not reach Bancho").await,
    };
    let (read_half, mut writer) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let login = format!("PASS {password}\r\nNICK {nick}\r\nUSER {nick} 0 * :{nick}\r\n");
    drop(password);
    if writer.write_all(login.as_bytes()).await.is_err() {
        return close(socket, "could not reach Bancho").await;
    }
    drop(login);

    let reason = run(&mut socket, &mut reader, &mut writer, &nick, &log).await;
    log.release_all();
    let _ = writer.write_all(b"QUIT\r\n").await;
    close(socket, reason).await;
}

enum Outgoing {
    Privmsg { target: String, text: String },
    Raw(String),
}

// Relays until either side goes away, returning why.
async fn run(
    socket: &mut WebSocket,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut OwnedWriteHalf,
    nick: &str,
    log: &RoomLog,
) -> &'static str {
    let login_deadline = Instant::now() + LOGIN_TIMEOUT;
    let mut registered = false;
    let mut queue: VecDeque<Outgoing> = VecDeque::new();
    let mut sent: VecDeque<Instant> = VecDeque::new();
    // read_until keeps a partly read line in `buf` when another branch wins the
    // select, so the next call picks up where it left off.
    let mut buf = Vec::new();

    loop {
        let now = Instant::now();
        while sent
            .front()
            .is_some_and(|&t| now.duration_since(t) >= SEND_WINDOW)
        {
            sent.pop_front();
        }
        let next_send = if sent.len() < SEND_LIMIT {
            now
        } else {
            sent[0] + SEND_WINDOW
        };

        tokio::select! {
            read = reader.read_until(b'\n', &mut buf) => {
                match read {
                    // A bad login gets a MOTD saying so, then Bancho hangs up.
                    Ok(0) | Err(_) if !registered => {
                        send(socket, &ServerMsg::LoginFailed).await;
                        return "login failed";
                    }
                    Ok(0) | Err(_) => return "Bancho closed the connection",
                    Ok(_) if !buf.ends_with(b"\n") => {
                        if buf.len() > LINE_MAX {
                            return "Bancho sent an invalid line";
                        }
                        continue;
                    }
                    Ok(_) => {}
                }
                let line = String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string();
                buf.clear();
                let Some(line) = parse(&line) else { continue };
                match line.command {
                    "PING" => {
                        let token = line.params.first().copied().unwrap_or_default();
                        if writer.write_all(format!("PONG :{token}\r\n").as_bytes()).await.is_err() {
                            return "Bancho closed the connection";
                        }
                    }
                    "001" => {
                        registered = true;
                        if !send(socket, &ServerMsg::Connected { nick: nick.to_string() }).await {
                            return "page closed";
                        }
                    }
                    "464" => {
                        send(socket, &ServerMsg::LoginFailed).await;
                        return "login failed";
                    }
                    "ERROR" => return "Bancho closed the connection",
                    _ => {
                        if let Some(msg) = to_server_msg(&line, nick) {
                            log.record(&msg).await;
                            if !send(socket, &msg).await {
                                return "page closed";
                            }
                        }
                    }
                }
            }
            msg = next_client_msg(socket) => {
                let Some(msg) = msg else { return "page closed" };
                if !registered {
                    continue;
                }
                match msg {
                    Ok(ClientMsg::Send { target, text }) => {
                        let text = clean_text(&text);
                        if valid_name(&target) && !text.trim().is_empty() {
                            queue.push_back(Outgoing::Privmsg { target, text });
                        }
                    }
                    Ok(ClientMsg::Join { channel }) if valid_name(&channel) => {
                        queue.push_back(Outgoing::Raw(format!("JOIN {channel}")));
                    }
                    Ok(ClientMsg::Part { channel }) if valid_name(&channel) => {
                        queue.push_back(Outgoing::Raw(format!("PART {channel}")));
                    }
                    _ => {}
                }
            }
            _ = sleep_until(next_send), if !queue.is_empty() => {
                let Some(out) = queue.pop_front() else { continue };
                sent.push_back(Instant::now());
                let line = match &out {
                    Outgoing::Privmsg { target, text } => format!("PRIVMSG {target} :{text}\r\n"),
                    Outgoing::Raw(raw) => format!("{raw}\r\n"),
                };
                if writer.write_all(line.as_bytes()).await.is_err() {
                    return "Bancho closed the connection";
                }
                if let Outgoing::Privmsg { target, text } = out {
                    let echo = ServerMsg::Message { from: nick.to_string(), target, text, action: false };
                    log.record(&echo).await;
                    if !send(socket, &echo).await {
                        return "page closed";
                    }
                }
            }
            _ = sleep_until(login_deadline), if !registered => return "Bancho didn't answer the login",
        }
    }
}

// Which relay connection saves each room's chat, by room id. Every referee in a
// room's channel sees the same lines, so only one connection saves them: the first
// to see a line there. When it leaves the channel, the next one to see a line takes over.
#[derive(Clone, Default)]
pub struct ChatLoggers(Arc<Mutex<HashMap<i64, u64>>>);

// Saves the lines said in multiplayer rooms (#mp_<id>) to room_chat_messages, for
// moderation. Private messages aren't saved.
struct RoomLog {
    db: PgPool,
    loggers: ChatLoggers,
    // Tells this connection apart in `loggers`.
    conn: u64,
    user_id: Uuid,
}

fn room_id(channel: &str) -> Option<i64> {
    channel.strip_prefix("#mp_")?.parse().ok()
}

impl RoomLog {
    async fn record(&self, msg: &ServerMsg) {
        match msg {
            ServerMsg::Message {
                from,
                target,
                text,
                action,
            } => {
                let Some(mp_id) = room_id(target) else { return };
                let claimed = {
                    let mut loggers = self.loggers.0.lock().unwrap();
                    *loggers.entry(mp_id).or_insert(self.conn) == self.conn
                };
                if !claimed {
                    return;
                }
                if let Err(err) =
                    db::chat::insert_message(&self.db, mp_id, from, text, *action, self.user_id)
                        .await
                {
                    tracing::error!(error = ?err, mp_id, "could not save a chat line");
                }
            }
            ServerMsg::Parted { channel } => {
                if let Some(mp_id) = room_id(channel) {
                    let mut loggers = self.loggers.0.lock().unwrap();
                    if loggers.get(&mp_id) == Some(&self.conn) {
                        loggers.remove(&mp_id);
                    }
                }
            }
            _ => {}
        }
    }

    fn release_all(&self) {
        self.loggers
            .0
            .lock()
            .unwrap()
            .retain(|_, c| *c != self.conn);
    }
}

struct Line<'a> {
    nick: Option<&'a str>,
    command: &'a str,
    params: Vec<&'a str>,
}

// Splits `:nick!user@host COMMAND a b :trailing text` into its parts.
fn parse(line: &str) -> Option<Line<'_>> {
    let mut rest = line;
    let mut prefix = None;
    if let Some(r) = rest.strip_prefix(':') {
        let (p, r) = r.split_once(' ')?;
        prefix = Some(p);
        rest = r;
    }
    let (head, trailing) = match rest.split_once(" :") {
        Some((head, trailing)) => (head, Some(trailing)),
        None => (rest, None),
    };
    let mut parts = head.split(' ').filter(|s| !s.is_empty());
    let command = parts.next()?;
    let mut params: Vec<&str> = parts.collect();
    params.extend(trailing);
    Some(Line {
        nick: prefix.map(|p| p.split('!').next().unwrap_or(p)),
        command,
        params,
    })
}

// The lines the page cares about. Bancho also sends a lot of JOIN/PART/QUIT noise
// about other users, which is dropped.
fn to_server_msg(line: &Line, nick: &str) -> Option<ServerMsg> {
    let from_me = line.nick.is_some_and(|n| n.eq_ignore_ascii_case(nick));
    match (line.command, line.params.as_slice()) {
        ("PRIVMSG", [target, text]) => {
            let (text, action) = match text
                .strip_prefix("\u{1}ACTION ")
                .map(|t| t.trim_end_matches('\u{1}'))
            {
                Some(t) => (t, true),
                None => (*text, false),
            };
            Some(ServerMsg::Message {
                from: line.nick?.to_string(),
                target: target.to_string(),
                text: text.to_string(),
                action,
            })
        }
        ("JOIN", [channel, ..]) if from_me => Some(ServerMsg::Joined {
            channel: channel.to_string(),
        }),
        ("PART", [channel, ..]) if from_me => Some(ServerMsg::Parted {
            channel: channel.to_string(),
        }),
        // No such nick / channel, can't send to channel, too many channels.
        ("401" | "403" | "404" | "405", [_, subject, message]) => Some(ServerMsg::Error {
            message: format!("{subject}: {message}"),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_privmsg() {
        let line = parse(":BanchoBot!cho@ppy.sh PRIVMSG #mp_1 :Slot 1  Ready").unwrap();
        assert_eq!(line.nick, Some("BanchoBot"));
        assert_eq!(line.command, "PRIVMSG");
        assert_eq!(line.params, ["#mp_1", "Slot 1  Ready"]);
    }

    #[test]
    fn parses_ping() {
        let line = parse("PING :cho.ppy.sh").unwrap();
        assert_eq!(line.nick, None);
        assert_eq!(line.params, ["cho.ppy.sh"]);
    }

    #[test]
    fn rejects_injection() {
        assert!(!valid_name("#mp_1 :x"));
        assert!(!valid_name("a\r\nQUIT"));
        assert_eq!(clean_text("hi\r\nQUIT"), "hi  QUIT");
    }
}
