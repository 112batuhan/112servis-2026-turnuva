//! Each referee's own connection to Bancho's IRC server, beside the site's bots and
//! behind the relay socket in handlers/irc.rs: browsers can't open raw TCP connections, so the page talks JSON over
//! a WebSocket and this module holds the IRC connection for it. Each user has at most one
//! Bancho connection (a Session), shared by all of their tabs: a referee running two
//! matches side by side is still one account to Bancho, sending at one account's pace.
//! It logs in with the referee's own IRC credentials, which live only in memory until
//! they're sent. It answers PINGs and paces outgoing lines.
//!
//! It's for talking: the rooms are made, run and logged by the site's bots
//! (manager.rs), and every page reads a room's chat from what its bot saved. A tab
//! that has a match open says so (`attend`); the room's bot then makes the referee one
//! of its referees and this session joins the room's channel, so what the referee
//! types goes to the room from their own account. Room lines aren't passed on to the
//! tabs (they have the bot's log); private messages are.

use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::tcp::OwnedWriteHalf,
    sync::{broadcast, mpsc},
    time::{sleep_until, Instant},
};
use uuid::Uuid;

use super::{
    irc::{connect, parse, privmsg_text, valid_name, Line, LINE_MAX, LOGIN_TIMEOUT},
    Bancho,
};
use crate::AppState;

// Bancho mutes accounts that send too fast, so at most SEND_LIMIT lines go out in any
// SEND_WINDOW. The rest wait their turn in order.
const SEND_LIMIT: usize = 8;
const SEND_WINDOW: Duration = Duration::from_secs(5);
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

// What the page receives.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMsg {
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
    // The account is one of the site's bots: logging in with it would take the bot off
    // Bancho.
    BotAccount,
    // A private message, including the ones we send (IRC doesn't echo those back).
    Message {
        from: String,
        target: String,
        text: String,
        action: bool,
        // Its number in the session, counting from 1.
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

pub enum Outgoing {
    Privmsg { target: String, text: String },
    Raw(String),
}

// What tabs ask of the session.
pub enum Command {
    Send(Outgoing),
    Close(String),
    Quit,
}

// A session as the room manager holds it: the room's bot lets the referee's connection
// into the room's channel once it has made them one of its referees.
#[derive(Clone)]
pub struct SessionLink {
    pub id: u32,
    // As it logged in to Bancho.
    pub nick: String,
    pub tx: mpsc::UnboundedSender<SessionRequest>,
}

// What the room manager asks of a session.
pub enum SessionRequest {
    Join(String),
    Part(String),
}

impl SessionLink {
    pub(super) fn send(&self, request: SessionRequest) {
        let _ = self.tx.send(request);
    }
}

// A user's Bancho connection, shared by their tabs.
pub struct Session {
    id: u32,
    nick: String,
    // Only tabs hold strong senders, so the session ends when the last tab goes away.
    commands: mpsc::WeakUnboundedSender<Command>,
    // What the room manager asks of it (joining a room's channel).
    requests: mpsc::UnboundedSender<SessionRequest>,
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
pub struct Attached {
    pub session: Arc<Session>,
    pub commands: mpsc::UnboundedSender<Command>,
    pub events: broadcast::Receiver<Arc<String>>,
    // Sent before anything from `events`.
    pub backlog: Vec<ServerMsg>,
}

impl Session {
    pub fn link(&self) -> SessionLink {
        SessionLink {
            id: self.id,
            nick: self.nick.clone(),
            tx: self.requests.clone(),
        }
    }

    fn attach(self: &Arc<Self>) -> Option<Attached> {
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
            session: self.clone(),
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
    pub fn attach(&self, user_id: Uuid) -> Option<Attached> {
        self.0.lock().unwrap().sessions.get(&user_id)?.attach()
    }

    // Joins the user's session, or starts one with these credentials. A tab that logs
    // in while another one is connected just joins it, whatever it typed.
    pub fn login(
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
        let (requests, requests_rx) = mpsc::unbounded_channel();
        let (events, _) = broadcast::channel(TAB_BUFFER);
        let session = Arc::new(Session {
            id: rand::random(),
            nick,
            commands: commands.downgrade(),
            requests,
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

        let addr = state.config.bancho_irc_addr.clone();
        tokio::spawn(run_session(
            self.clone(),
            state.bancho.clone(),
            user_id,
            session,
            password,
            addr,
            rx,
            requests_rx,
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

#[allow(clippy::too_many_arguments)]
async fn run_session(
    users: IrcUsers,
    rooms: Bancho,
    user_id: Uuid,
    session: Arc<Session>,
    password: String,
    addr: String,
    mut commands: mpsc::UnboundedReceiver<Command>,
    mut requests: mpsc::UnboundedReceiver<SessionRequest>,
) {
    let reason = bancho(
        &users,
        user_id,
        &session,
        password,
        &addr,
        &mut commands,
        &mut requests,
    )
    .await;
    // No tab can join from here on; the ones still here get the reason and are closed.
    users.end(user_id, &session);
    rooms.session_ended(session.id);
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
    requests: &mut mpsc::UnboundedReceiver<SessionRequest>,
) -> &'static str {
    let connected = connect(addr, &session.nick, &password).await;
    drop(password);
    let Ok((mut reader, mut writer)) = connected else {
        users.refund_login(user_id);
        return "could not reach Bancho";
    };

    let reason = run(
        users,
        user_id,
        session,
        &mut reader,
        &mut writer,
        commands,
        requests,
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
    requests: &mut mpsc::UnboundedReceiver<SessionRequest>,
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
            Some(request) = requests.recv() => {
                let (verb, channel) = match request {
                    SessionRequest::Join(channel) => ("JOIN", channel),
                    SessionRequest::Part(channel) => ("PART", channel),
                };
                if registered && valid_name(&channel) {
                    queue.push_back(Outgoing::Raw(format!("{verb} {channel}")));
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
                // The room's bot logs what's said in rooms; private messages are echoed here.
                if let Outgoing::Privmsg { target, text } = out {
                    if !target.starts_with('#') {
                        session.emit(&ServerMsg::Message { from: nick.to_string(), target, text, action: false, seq: None });
                    }
                }
            }
            _ = sleep_until(login_deadline), if !registered => return "Bancho didn't answer the login",
        }
    }
}

// The lines the page cares about: private messages, our own joins and parts, and
// refusals. Room lines are the bots' to log, and Bancho also sends a lot of
// JOIN/PART/QUIT noise about other users; both are dropped.
fn to_server_msg(line: &Line, nick: &str) -> Option<ServerMsg> {
    let from_me = line.nick.is_some_and(|n| n.eq_ignore_ascii_case(nick));
    match (line.command, line.params.as_slice()) {
        ("PRIVMSG", [target, _]) if target.starts_with('#') => None,
        ("PRIVMSG", [target, text]) => {
            let (text, action) = privmsg_text(text);
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
        let session = Arc::new(Session {
            id: 1,
            nick: "Ref".into(),
            commands: commands.downgrade(),
            requests: mpsc::unbounded_channel().0,
            state: Mutex::new(SessionState {
                events: broadcast::channel(TAB_BUFFER).0,
                registered: false,
                channels: BTreeSet::new(),
                private: VecDeque::new(),
                private_count: 0,
            }),
        });
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
        // Private messages are kept for later tabs.
        session.emit(&message("Ref"));
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
