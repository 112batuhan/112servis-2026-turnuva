//! A relay between the referee pages and Bancho's IRC server. Browsers can't open raw
//! TCP connections, so the page talks JSON over a WebSocket and this module holds the
//! IRC connection for it. Each user has at most one Bancho connection (a Session),
//! shared by all of their tabs: a referee running two matches side by side is still one
//! account to Bancho, sending at one account's pace. It logs in with the referee's own
//! IRC credentials, which live only in memory until they're sent. It answers PINGs,
//! paces outgoing lines and saves what's said in multiplayer rooms (see RoomLog);
//! everything Bancho says goes to every tab, and the pages do all the match logic.

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
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
    sync::{
        broadcast::{self, error::RecvError},
        mpsc,
    },
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
// Every referee's connection comes from this server's address, so one person mistyping
// their IRC password over and over could get it throttled by Bancho for everyone. After
// FAILED_LOGIN_LIMIT failed logins a user waits LOGIN_COOLDOWN from the last one.
const FAILED_LOGIN_LIMIT: u32 = 3;
const LOGIN_COOLDOWN: Duration = Duration::from_secs(5 * 60);
// Messages a tab can fall behind by before it's dropped.
const TAB_BUFFER: usize = 1024;
// Private messages a session keeps for tabs that join later. Only in memory, for as
// long as the session lasts; unlike room chat they're never saved.
const PRIVATE_HISTORY: usize = 500;

// What the page sends. The first message must be `Login` or `Resume`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMsg {
    // Joins the user's Bancho connection if another tab has one, otherwise logs in.
    Login { username: String, password: String },
    // Joins the user's Bancho connection if another tab has one.
    Resume,
    Send { target: String, text: String },
    Join { channel: String },
    Part { channel: String },
    // Closes a private conversation in every tab and forgets its history.
    Close { name: String },
    // Ends the Bancho connection, for every tab.
    Quit,
}

// What the page receives.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMsg {
    // Bancho accepted the login. `session` tells connections apart, for `seq`.
    Connected {
        nick: String,
        session: u32,
    },
    // Bancho rejected the username or IRC password.
    LoginFailed,
    // Too many failed logins; the user may try again in `retry_after` seconds.
    LoginBlocked {
        retry_after: u64,
    },
    // `Resume` found no connection to join.
    NoSession,
    // A chat line, including the ones we send (IRC doesn't echo those back).
    Message {
        from: String,
        target: String,
        text: String,
        action: bool,
        // Private messages only: their number in the session, counting from 1.
        #[serde(skip_serializing_if = "Option::is_none")]
        seq: Option<u64>,
    },
    // A private message from before this tab joined; `at` is in unix ms. A tab that
    // rejoins the same session skips the ones it already has by `seq`.
    PastMessage {
        from: String,
        target: String,
        text: String,
        action: bool,
        at: i64,
        seq: u64,
    },
    Joined {
        channel: String,
    },
    Parted {
        channel: String,
    },
    // A tab closed this private conversation; a new message opens it again.
    ConversationClosed {
        name: String,
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
    let user_id = user.id;
    Ok(ws.on_upgrade(move |socket| tab(socket, state, user_id)))
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

// One tab's WebSocket: joins or starts the user's session, then passes messages both
// ways until the tab or the session goes away.
async fn tab(mut socket: WebSocket, state: AppState, user_id: Uuid) {
    let attached = match timeout(LOGIN_TIMEOUT, next_client_msg(&mut socket)).await {
        Ok(Some(Ok(ClientMsg::Resume))) => match state.irc_users.attach(user_id) {
            Some(attached) => attached,
            None => {
                send(&mut socket, &ServerMsg::NoSession).await;
                return close(socket, "no connection to resume").await;
            }
        },
        Ok(Some(Ok(ClientMsg::Login { username, password }))) => {
            // osu! usernames use underscores for spaces on IRC.
            let nick = username.trim().replace(' ', "_");
            if !valid_name(&nick) || password.is_empty() || password.chars().any(|c| c.is_control())
            {
                send(&mut socket, &ServerMsg::LoginFailed).await;
                return close(socket, "invalid login").await;
            }
            match state.irc_users.login(&state, user_id, nick, password) {
                Ok(attached) => attached,
                Err(wait) => {
                    let retry_after = wait.as_secs().max(1);
                    send(&mut socket, &ServerMsg::LoginBlocked { retry_after }).await;
                    return close(socket, "too many failed logins").await;
                }
            }
        }
        _ => return close(socket, "login expected").await,
    };
    pump(socket, attached).await;
}

async fn pump(mut socket: WebSocket, mut tab: Attached) {
    for msg in &tab.backlog {
        if !send(&mut socket, msg).await {
            return;
        }
    }
    loop {
        tokio::select! {
            event = tab.events.recv() => match event {
                Ok(json) => {
                    if socket.send(Message::Text(json.as_ref().clone())).await.is_err() {
                        return;
                    }
                }
                Err(RecvError::Lagged(_)) => return close(socket, "this tab fell behind").await,
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
                    Ok(ClientMsg::Join { channel }) if valid_name(&channel) => {
                        Some(Command::Send(Outgoing::Raw(format!("JOIN {channel}"))))
                    }
                    Ok(ClientMsg::Part { channel }) if valid_name(&channel) => {
                        Some(Command::Send(Outgoing::Raw(format!("PART {channel}"))))
                    }
                    Ok(ClientMsg::Close { name }) if valid_name(&name) && !name.starts_with('#') => {
                        Some(Command::Close(name))
                    }
                    Ok(ClientMsg::Quit) => Some(Command::Quit),
                    _ => None,
                };
                if let Some(command) = command {
                    let _ = tab.commands.send(command);
                }
            }
        }
    }
    let _ = socket.close().await;
}

enum Outgoing {
    Privmsg { target: String, text: String },
    Raw(String),
}

// What tabs ask of the session.
enum Command {
    Send(Outgoing),
    Close(String),
    Quit,
}

// A user's Bancho connection, shared by their tabs.
struct Session {
    id: u32,
    nick: String,
    // Only tabs hold strong senders, so the session ends when the last tab goes away.
    commands: mpsc::WeakUnboundedSender<Command>,
    state: Mutex<SessionState>,
}

struct SessionState {
    events: broadcast::Sender<Arc<String>>,
    // What a tab that joins later needs to catch up.
    registered: bool,
    channels: BTreeSet<String>,
    // Oldest first, at most PRIVATE_HISTORY.
    private: VecDeque<PrivateLine>,
    private_count: u64,
}

struct PrivateLine {
    from: String,
    target: String,
    text: String,
    action: bool,
    at: i64,
    seq: u64,
}

fn unix_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

// A tab's handle on its session.
struct Attached {
    commands: mpsc::UnboundedSender<Command>,
    events: broadcast::Receiver<Arc<String>>,
    // Sent before anything from `events`.
    backlog: Vec<ServerMsg>,
}

impl Session {
    fn attach(&self) -> Option<Attached> {
        let commands = self.commands.upgrade()?;
        let state = self.state.lock().unwrap();
        let mut backlog = Vec::new();
        if state.registered {
            backlog.push(ServerMsg::Connected {
                nick: self.nick.clone(),
                session: self.id,
            });
            backlog.extend(state.channels.iter().map(|channel| ServerMsg::Joined {
                channel: channel.clone(),
            }));
            backlog.extend(state.private.iter().map(|l| ServerMsg::PastMessage {
                from: l.from.clone(),
                target: l.target.clone(),
                text: l.text.clone(),
                action: l.action,
                at: l.at,
                seq: l.seq,
            }));
        }
        Some(Attached {
            commands,
            events: state.events.subscribe(),
            backlog,
        })
    }

    // Forgets a private conversation (by the other person's nick) and tells every tab.
    fn close_conversation(&self, name: &str) {
        let mut state = self.state.lock().unwrap();
        let nick = &self.nick;
        state.private.retain(|l| {
            let other = if l.from.eq_ignore_ascii_case(nick) {
                &l.target
            } else {
                &l.from
            };
            !other.eq_ignore_ascii_case(name)
        });
        let msg = ServerMsg::ConversationClosed {
            name: name.to_string(),
        };
        if let Ok(json) = serde_json::to_string(&msg) {
            let _ = state.events.send(Arc::new(json));
        }
    }

    // Sends a message to every tab. Under the same lock as `attach`, so a tab joining
    // meanwhile gets each change either in its backlog or as an event.
    fn emit(&self, msg: &ServerMsg) {
        let mut state = self.state.lock().unwrap();
        match msg {
            ServerMsg::Connected { .. } => state.registered = true,
            ServerMsg::Joined { channel } => {
                state.channels.insert(channel.clone());
            }
            ServerMsg::Parted { channel } => {
                state.channels.remove(channel);
            }
            ServerMsg::Message {
                from,
                target,
                text,
                action,
                ..
            } if !target.starts_with('#') => {
                state.private_count += 1;
                let seq = state.private_count;
                if state.private.len() == PRIVATE_HISTORY {
                    state.private.pop_front();
                }
                state.private.push_back(PrivateLine {
                    from: from.clone(),
                    target: target.clone(),
                    text: text.clone(),
                    action: *action,
                    at: unix_ms(),
                    seq,
                });
                let numbered = ServerMsg::Message {
                    from: from.clone(),
                    target: target.clone(),
                    text: text.clone(),
                    action: *action,
                    seq: Some(seq),
                };
                if let Ok(json) = serde_json::to_string(&numbered) {
                    let _ = state.events.send(Arc::new(json));
                }
                return;
            }
            _ => {}
        }
        if let Ok(json) = serde_json::to_string(msg) {
            let _ = state.events.send(Arc::new(json));
        }
    }
}

// Each user's session and recent failed logins.
#[derive(Clone, Default)]
pub struct IrcUsers(Arc<Mutex<IrcUsersInner>>);

#[derive(Default)]
struct IrcUsersInner {
    sessions: HashMap<Uuid, Arc<Session>>,
    failures: HashMap<Uuid, FailedLogins>,
}

struct FailedLogins {
    count: u32,
    last: Instant,
}

impl IrcUsersInner {
    // Counts a login attempt as failed until Bancho accepts it, so attempts made in
    // parallel can't get past the limit. Err holds how long the user has to wait.
    fn begin_login(&mut self, user_id: Uuid) -> Result<(), Duration> {
        let now = Instant::now();
        self.failures
            .retain(|_, f| now.duration_since(f.last) < LOGIN_COOLDOWN);
        let failures = self.failures.entry(user_id).or_insert(FailedLogins {
            count: 0,
            last: now,
        });
        if failures.count >= FAILED_LOGIN_LIMIT {
            return Err(LOGIN_COOLDOWN - now.duration_since(failures.last));
        }
        failures.count += 1;
        failures.last = now;
        Ok(())
    }
}

impl IrcUsers {
    fn attach(&self, user_id: Uuid) -> Option<Attached> {
        self.0.lock().unwrap().sessions.get(&user_id)?.attach()
    }

    // Joins the user's session, or starts one with these credentials. A tab that logs
    // in while another one is connected just joins it, whatever it typed.
    fn login(
        &self,
        state: &AppState,
        user_id: Uuid,
        nick: String,
        password: String,
    ) -> Result<Attached, Duration> {
        let mut inner = self.0.lock().unwrap();
        if let Some(attached) = inner.sessions.get(&user_id).and_then(|s| s.attach()) {
            return Ok(attached);
        }
        inner.begin_login(user_id)?;

        let (commands, rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(TAB_BUFFER);
        let session = Arc::new(Session {
            id: rand::random(),
            nick,
            commands: commands.downgrade(),
            state: Mutex::new(SessionState {
                events,
                registered: false,
                channels: BTreeSet::new(),
                private: VecDeque::new(),
                private_count: 0,
            }),
        });
        let attached = session.attach().expect("the sender is alive");
        drop(commands);
        inner.sessions.insert(user_id, session.clone());

        let log = RoomLog {
            db: state.db.clone(),
            loggers: state.chat_loggers.clone(),
            conn: rand::random(),
            user_id,
        };
        let addr = state.config.bancho_irc_addr.clone();
        tokio::spawn(run_session(
            self.clone(),
            user_id,
            session,
            password,
            addr,
            rx,
            log,
        ));
        Ok(attached)
    }

    // The credentials never reached Bancho, so the attempt doesn't count.
    fn refund_login(&self, user_id: Uuid) {
        let mut inner = self.0.lock().unwrap();
        if let Some(failures) = inner.failures.get_mut(&user_id) {
            failures.count = failures.count.saturating_sub(1);
        }
    }

    fn logged_in(&self, user_id: Uuid) {
        self.0.lock().unwrap().failures.remove(&user_id);
    }

    fn end(&self, user_id: Uuid, session: &Arc<Session>) {
        let mut inner = self.0.lock().unwrap();
        if inner
            .sessions
            .get(&user_id)
            .is_some_and(|s| Arc::ptr_eq(s, session))
        {
            inner.sessions.remove(&user_id);
        }
    }
}

async fn run_session(
    users: IrcUsers,
    user_id: Uuid,
    session: Arc<Session>,
    password: String,
    addr: String,
    mut commands: mpsc::UnboundedReceiver<Command>,
    log: RoomLog,
) {
    let reason = bancho(
        &users,
        user_id,
        &session,
        password,
        &addr,
        &mut commands,
        &log,
    )
    .await;
    // No tab can join from here on; the ones still here get the reason and are closed.
    users.end(user_id, &session);
    log.release_all();
    session.emit(&ServerMsg::Closed {
        reason: reason.to_string(),
    });
}

async fn bancho(
    users: &IrcUsers,
    user_id: Uuid,
    session: &Session,
    password: String,
    addr: &str,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    log: &RoomLog,
) -> &'static str {
    let stream = match timeout(LOGIN_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "could not reach Bancho");
            users.refund_login(user_id);
            return "could not reach Bancho";
        }
        Err(_) => {
            users.refund_login(user_id);
            return "could not reach Bancho";
        }
    };
    let (read_half, mut writer) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    let nick = &session.nick;
    let login = format!("PASS {password}\r\nNICK {nick}\r\nUSER {nick} 0 * :{nick}\r\n");
    drop(password);
    if writer.write_all(login.as_bytes()).await.is_err() {
        return "could not reach Bancho";
    }
    drop(login);

    let reason = run(
        users,
        user_id,
        session,
        &mut reader,
        &mut writer,
        commands,
        log,
    )
    .await;
    let _ = writer.write_all(b"QUIT\r\n").await;
    reason
}

// Relays until Bancho goes away or the tabs are done with it, returning why.
async fn run(
    users: &IrcUsers,
    user_id: Uuid,
    session: &Session,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut OwnedWriteHalf,
    commands: &mut mpsc::UnboundedReceiver<Command>,
    log: &RoomLog,
) -> &'static str {
    let nick = session.nick.as_str();
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
                        session.emit(&ServerMsg::LoginFailed);
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
                        users.logged_in(user_id);
                        session.emit(&ServerMsg::Connected { nick: nick.to_string(), session: session.id });
                    }
                    "464" => {
                        session.emit(&ServerMsg::LoginFailed);
                        return "login failed";
                    }
                    "ERROR" => return "Bancho closed the connection",
                    _ => {
                        if let Some(msg) = to_server_msg(&line, nick) {
                            log.record(&msg).await;
                            session.emit(&msg);
                        }
                    }
                }
            }
            command = commands.recv() => match command {
                None => return "every tab closed",
                Some(Command::Quit) => return "disconnected in another tab",
                Some(Command::Close(name)) => session.close_conversation(&name),
                // Bancho ignores everything before the login goes through.
                Some(Command::Send(out)) if registered => queue.push_back(out),
                Some(Command::Send(_)) => {}
            },
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
                    let echo = ServerMsg::Message { from: nick.to_string(), target, text, action: false, seq: None };
                    log.record(&echo).await;
                    session.emit(&echo);
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
                ..
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
                seq: None,
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

    #[test]
    fn limits_failed_logins() {
        let users = IrcUsers::default();
        let user = Uuid::from_u128(rand::random());
        let begin = |user| users.0.lock().unwrap().begin_login(user);
        for _ in 0..FAILED_LOGIN_LIMIT {
            assert!(begin(user).is_ok());
        }
        assert!(begin(user).is_err());
        // One that never reached Bancho gives its attempt back.
        users.refund_login(user);
        assert!(begin(user).is_ok());
        // Logging in forgets the failures, and other users aren't affected.
        assert!(begin(Uuid::from_u128(rand::random())).is_ok());
        users.logged_in(user);
        assert!(begin(user).is_ok());
    }

    #[test]
    fn later_tabs_catch_up() {
        let (commands, _rx) = mpsc::unbounded_channel();
        let session = Session {
            id: 1,
            nick: "Ref".into(),
            commands: commands.downgrade(),
            state: Mutex::new(SessionState {
                events: broadcast::channel(TAB_BUFFER).0,
                registered: false,
                channels: BTreeSet::new(),
                private: VecDeque::new(),
                private_count: 0,
            }),
        };
        let mut first = session.attach().unwrap();
        session.emit(&ServerMsg::Connected {
            nick: "Ref".into(),
            session: 1,
        });
        for channel in ["#mp_1", "#mp_2"] {
            session.emit(&ServerMsg::Joined {
                channel: channel.into(),
            });
        }
        session.emit(&ServerMsg::Parted {
            channel: "#mp_1".into(),
        });
        let message = |target: &str| ServerMsg::Message {
            from: "BanchoBot".into(),
            target: target.into(),
            text: "hi".into(),
            action: false,
            seq: None,
        };
        // Private messages are kept for later tabs, room lines aren't.
        session.emit(&message("Ref"));
        session.emit(&message("#mp_2"));
        assert!(first.events.try_recv().unwrap().contains("connected"));

        let later = session.attach().unwrap();
        let backlog: Vec<_> = later
            .backlog
            .iter()
            .map(|m| serde_json::to_string(m).unwrap())
            .collect();
        assert_eq!(backlog.len(), 3);
        assert!(backlog[0].contains("connected"));
        assert!(backlog[1].contains("#mp_2"));
        assert!(backlog[2].contains("past_message") && backlog[2].contains(r#""seq":1"#));

        // Closing the conversation drops it from the history and tells the tabs.
        session.close_conversation("banchobot");
        let mut last = String::new();
        while let Ok(event) = first.events.try_recv() {
            last = event.to_string();
        }
        assert_eq!(last, r#"{"type":"conversation_closed","name":"banchobot"}"#);
        assert_eq!(session.attach().unwrap().backlog.len(), 2);

        // Once every tab is gone, nothing can join.
        drop((commands, first, later));
        assert!(session.attach().is_none());
    }
}
