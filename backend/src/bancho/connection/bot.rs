//! One bot account's connection to Bancho. It logs in, answers PINGs, joins and leaves
//! channels, and sends what the manager queues at a careful pace: a bot runs several
//! rooms at once, so a mute would stop all of them. Everything it hears goes to the
//! manager, which does the rest. It reconnects by itself, except after a refused login.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use time::OffsetDateTime;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt},
    sync::mpsc,
    time::{sleep, sleep_until, timeout, Instant},
};

use crate::bancho::names::irc_name;

use super::irc::{
    self, clean_text, parse, privmsg_text, room_id, valid_name, Pacer, LINE_MAX, LOGIN_TIMEOUT,
};

// The pace a bot keeps, below what Bancho allows: at most SEND_LIMIT lines
// in any SEND_WINDOW, never two closer than SEND_GAP.
pub const SEND_LIMIT: usize = 6;
pub const SEND_WINDOW: Duration = Duration::from_secs(5);
const SEND_GAP: Duration = Duration::from_millis(400);
// Lines a room can have waiting; past that its least important ones are dropped.
const ROOM_QUEUE_MAX: usize = 40;
// After this long without a word from Bancho the bot pings it, and this long means the line
// is dead.
const KEEPALIVE: Duration = Duration::from_secs(60);
const IDLE_TIMEOUT: Duration = Duration::from_secs(180);
// Between reconnects, doubling up to the max.
const RETRY_MIN: Duration = Duration::from_secs(5);
const RETRY_MAX: Duration = Duration::from_secs(300);

// How soon a line should go out. Within a priority, rooms take turns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    // A refresh nobody is waiting on (!mp settings as a map starts).
    Low,
    // What the bot does by itself (placing players, kicking, adding referees).
    Normal,
    // What a referee asked for just now.
    High,
}

pub enum BotCmd {
    Send {
        target: String,
        text: String,
        room: Option<i64>,
        priority: Priority,
    },
    Join(String),
    Part(String),
    Stop,
}

#[derive(Debug)]
pub enum BotEvent {
    Online {
        conn: u64,
    },
    Offline {
        conn: u64,
        reason: String,
        // Bancho refused the password: the bot waits until a host fixes it.
        login_failed: bool,
    },
    // A chat line, ours included (IRC doesn't echo what we send).
    Line {
        conn: u64,
        from: String,
        target: String,
        text: String,
        action: bool,
        at: OffsetDateTime,
    },
    Joined {
        conn: u64,
        channel: String,
    },
    Parted {
        conn: u64,
        channel: String,
    },
    // Bancho refused a command: no such channel, can't send there.
    Refused {
        conn: u64,
        code: String,
        subject: String,
        message: String,
    },
}

// What the bots page shows about the line, read without asking the task.
#[derive(Debug, Default, Clone)]
pub struct LineStats {
    pub queued: usize,
    pub recent: usize,
}

pub struct BotConn {
    pub tx: mpsc::UnboundedSender<BotCmd>,
    pub stats: Arc<Mutex<LineStats>>,
}

// `conn` tells this connection's events apart from an earlier one's of the same bot.
pub fn spawn(
    conn: u64,
    nick: String,
    password: String,
    addr: String,
    events: mpsc::UnboundedSender<BotEvent>,
) -> BotConn {
    let (tx, rx) = mpsc::unbounded_channel();
    let stats = Arc::new(Mutex::new(LineStats::default()));
    tokio::spawn(run(
        conn,
        irc_name(&nick),
        password,
        addr,
        rx,
        events,
        stats.clone(),
    ));
    BotConn { tx, stats }
}

struct Queued {
    raw: String,
    // The chat line it is, to log it once sent.
    privmsg: Option<(String, String)>,
    room: Option<i64>,
    priority: Priority,
}

// The lines waiting to go out.
#[derive(Default)]
struct Outbox {
    lines: VecDeque<Queued>,
    last_room: Option<i64>,
}

impl Outbox {
    fn push(&mut self, mut line: Queued) {
        // One !mp settings per room is enough, as long as it goes after everything it should
        // read: a waiting one moves to the back instead of going twice.
        let settings = |q: &Queued| q.privmsg.as_ref().is_some_and(|(_, t)| t == "!mp settings");
        if settings(&line) {
            if let Some(i) = self
                .lines
                .iter()
                .position(|q| q.room == line.room && settings(q))
            {
                let waiting = self.lines.remove(i).expect("found");
                line.priority = line.priority.max(waiting.priority);
            }
        }
        if let Some(room) = line.room {
            let waiting = self.lines.iter().filter(|q| q.room == Some(room)).count();
            if waiting >= ROOM_QUEUE_MAX {
                let lowest = self
                    .lines
                    .iter()
                    .enumerate()
                    .filter(|(_, q)| q.room == Some(room))
                    .min_by_key(|(_, q)| q.priority)
                    .map(|(i, _)| i);
                if let Some(i) = lowest {
                    let dropped = self.lines.remove(i);
                    tracing::warn!(room, line = ?dropped.map(|q| q.raw), "bot queue full, dropped a line");
                }
            }
        }
        self.lines.push_back(line);
    }

    // The most important line, the next room's turn first among equals.
    fn pop(&mut self) -> Option<Queued> {
        let top = self.lines.iter().map(|q| q.priority).max()?;
        let pick = self
            .lines
            .iter()
            .position(|q| q.priority == top && (q.room.is_none() || q.room != self.last_room))
            .or_else(|| self.lines.iter().position(|q| q.priority == top))?;
        let line = self.lines.remove(pick)?;
        self.last_room = line.room;
        Some(line)
    }
}

async fn run(
    conn: u64,
    nick: String,
    password: String,
    addr: String,
    mut commands: mpsc::UnboundedReceiver<BotCmd>,
    events: mpsc::UnboundedSender<BotEvent>,
    stats: Arc<Mutex<LineStats>>,
) {
    let mut retry = RETRY_MIN;
    loop {
        let started = Instant::now();
        let end = session(
            conn,
            &nick,
            &password,
            &addr,
            &mut commands,
            &events,
            &stats,
        )
        .await;
        *stats.lock().unwrap() = LineStats::default();
        let (reason, login_failed, stop) = match end {
            End::Stopped => ("turned off".to_string(), false, true),
            End::LoginFailed => ("Bancho refused the login".to_string(), true, false),
            End::Lost(reason) => (reason.to_string(), false, false),
        };
        let _ = events.send(BotEvent::Offline {
            conn,
            reason,
            login_failed,
        });
        // A wrong password is tried again only when a host changes it (a new task).
        if stop || login_failed {
            return;
        }
        if started.elapsed() > RETRY_MAX {
            retry = RETRY_MIN;
        }
        // Whatever was asked while it was down is stale by the time it's back.
        let wait = sleep(retry);
        tokio::pin!(wait);
        loop {
            tokio::select! {
                _ = &mut wait => break,
                cmd = commands.recv() => match cmd {
                    None | Some(BotCmd::Stop) => return,
                    Some(_) => {}
                },
            }
        }
        retry = (retry * 2).min(RETRY_MAX);
    }
}

enum End {
    Stopped,
    LoginFailed,
    Lost(&'static str),
}

async fn session(
    conn: u64,
    nick: &str,
    password: &str,
    addr: &str,
    commands: &mut mpsc::UnboundedReceiver<BotCmd>,
    events: &mpsc::UnboundedSender<BotEvent>,
    stats: &Mutex<LineStats>,
) -> End {
    let Ok((mut reader, mut writer)) = irc::connect(addr, nick, password).await else {
        return End::Lost("could not reach Bancho");
    };
    let login_deadline = Instant::now() + LOGIN_TIMEOUT;
    let mut registered = false;
    let mut outbox = Outbox::default();
    let mut pacer = Pacer::new(SEND_LIMIT, SEND_WINDOW, SEND_GAP);
    let mut last_heard = Instant::now();
    let mut pinged = false;
    let mut buf = Vec::new();

    let end = loop {
        let now = Instant::now();
        let next_send = pacer.next_send(now);
        {
            let mut s = stats.lock().unwrap();
            s.queued = outbox.lines.len();
            s.recent = pacer.recent(now);
        }
        tokio::select! {
            read = reader.read_until(b'\n', &mut buf) => {
                match read {
                    // A bad login gets a MOTD saying so, then Bancho hangs up.
                    Ok(0) | Err(_) if !registered => break End::LoginFailed,
                    Ok(0) | Err(_) => break End::Lost("Bancho closed the connection"),
                    Ok(_) if !buf.ends_with(b"\n") => {
                        if buf.len() > LINE_MAX {
                            break End::Lost("Bancho sent an invalid line");
                        }
                        continue;
                    }
                    Ok(_) => {}
                }
                last_heard = Instant::now();
                pinged = false;
                let line = String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string();
                buf.clear();
                let Some(line) = parse(&line) else { continue };
                let from_me = line.nick.is_some_and(|n| n.eq_ignore_ascii_case(nick));
                match (line.command, line.params.as_slice()) {
                    ("PING", params) => {
                        let token = params.first().copied().unwrap_or_default();
                        if writer.write_all(format!("PONG :{token}\r\n").as_bytes()).await.is_err() {
                            break End::Lost("Bancho closed the connection");
                        }
                    }
                    ("001", _) => {
                        registered = true;
                        let _ = events.send(BotEvent::Online { conn });
                    }
                    ("464", _) => break End::LoginFailed,
                    ("ERROR", _) => break End::Lost("Bancho closed the connection"),
                    ("PRIVMSG", [target, text]) => {
                        let Some(from) = line.nick else { continue };
                        let (text, action) = privmsg_text(text);
                        let _ = events.send(BotEvent::Line {
                            conn,
                            from: from.to_string(),
                            target: target.to_string(),
                            text: text.to_string(),
                            action,
                            at: OffsetDateTime::now_utc(),
                        });
                    }
                    ("JOIN", [channel, ..]) if from_me => {
                        let _ = events.send(BotEvent::Joined { conn, channel: channel.to_string() });
                    }
                    ("PART", [channel, ..]) if from_me => {
                        let _ = events.send(BotEvent::Parted { conn, channel: channel.to_string() });
                    }
                    // No such nick / channel, can't send to channel, too many channels.
                    (code @ ("401" | "403" | "404" | "405"), [_, subject, message]) => {
                        let _ = events.send(BotEvent::Refused {
                            conn,
                            code: code.to_string(),
                            subject: subject.to_string(),
                            message: message.to_string(),
                        });
                    }
                    _ => {}
                }
            }
            cmd = commands.recv() => match cmd {
                None | Some(BotCmd::Stop) => break End::Stopped,
                // Bancho ignores everything before the login goes through.
                Some(_) if !registered => {}
                Some(BotCmd::Send { target, text, room, priority }) => {
                    let text = clean_text(&text);
                    if valid_name(&target) && !text.trim().is_empty() {
                        outbox.push(Queued {
                            raw: format!("PRIVMSG {target} :{text}"),
                            privmsg: Some((target, text)),
                            room,
                            priority,
                        });
                    }
                }
                Some(BotCmd::Join(channel)) if valid_name(&channel) => outbox.push(Queued {
                    room: room_id(&channel),
                    raw: format!("JOIN {channel}"),
                    privmsg: None,
                    priority: Priority::High,
                }),
                Some(BotCmd::Part(channel)) if valid_name(&channel) => outbox.push(Queued {
                    room: room_id(&channel),
                    raw: format!("PART {channel}"),
                    privmsg: None,
                    priority: Priority::Normal,
                }),
                Some(_) => {}
            },
            _ = sleep_until(next_send), if !outbox.lines.is_empty() => {
                let Some(out) = outbox.pop() else { continue };
                pacer.record(Instant::now());
                if timeout(LOGIN_TIMEOUT, writer.write_all(format!("{}\r\n", out.raw).as_bytes())).await.map_or(true, |r| r.is_err()) {
                    break End::Lost("Bancho closed the connection");
                }
                if let Some((target, text)) = out.privmsg {
                    let _ = events.send(BotEvent::Line {
                        conn,
                        from: nick.to_string(),
                        target,
                        text,
                        action: false,
                        at: OffsetDateTime::now_utc(),
                    });
                }
            }
            _ = sleep_until(login_deadline), if !registered => break End::Lost("Bancho didn't answer the login"),
            _ = sleep_until(last_heard + KEEPALIVE), if registered && !pinged => {
                pinged = true;
                if writer.write_all(b"PING :keepalive\r\n").await.is_err() {
                    break End::Lost("Bancho closed the connection");
                }
            }
            _ = sleep_until(last_heard + IDLE_TIMEOUT) => break End::Lost("Bancho went quiet"),
        }
    };
    let _ = writer.write_all(b"QUIT\r\n").await;
    end
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str, room: i64, priority: Priority) -> Queued {
        Queued {
            raw: text.into(),
            privmsg: Some((format!("#mp_{room}"), text.into())),
            room: Some(room),
            priority,
        }
    }

    #[test]
    fn rooms_take_turns_by_priority() {
        let mut out = Outbox::default();
        out.push(line("a1", 1, Priority::Normal));
        out.push(line("a2", 1, Priority::Normal));
        out.push(line("b1", 2, Priority::Normal));
        out.push(line("start", 3, Priority::High));
        let order: Vec<String> = std::iter::from_fn(|| out.pop().map(|q| q.raw)).collect();
        assert_eq!(order, ["start", "a1", "b1", "a2"]);
    }

    #[test]
    fn one_settings_per_room_after_the_rest() {
        let mut out = Outbox::default();
        out.push(line("!mp settings", 1, Priority::Normal));
        out.push(line("!mp move a 1", 1, Priority::Normal));
        out.push(line("!mp settings", 1, Priority::Normal));
        out.push(line("!mp settings", 2, Priority::Low));
        let order: Vec<String> = out.lines.iter().map(|q| q.raw.clone()).collect();
        assert_eq!(order, ["!mp move a 1", "!mp settings", "!mp settings"]);
    }
}
