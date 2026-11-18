//! What the bots share about speaking IRC to Bancho: splitting lines, keeping what
//! we send clean, logging in, and names.

use std::{collections::VecDeque, time::Duration};

use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::{timeout, Instant},
};

// Leaves room for the prefix Bancho adds within IRC's 512-byte line limit.
pub const TEXT_MAX: usize = 400;
// A line longer than this is not IRC; drop the connection rather than buffer it.
pub const LINE_MAX: usize = 8192;
// How long Bancho gets to answer the connection and the login.
pub const LOGIN_TIMEOUT: Duration = Duration::from_secs(20);

pub const BANCHO_BOT: &str = "BanchoBot";

pub struct Line<'a> {
    pub nick: Option<&'a str>,
    pub command: &'a str,
    pub params: Vec<&'a str>,
}

// Splits `:nick!user@host COMMAND a b :trailing text` into its parts.
pub fn parse(line: &str) -> Option<Line<'_>> {
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

// A PRIVMSG's text and whether it's a /me line.
pub fn privmsg_text(text: &str) -> (&str, bool) {
    match text
        .strip_prefix("\u{1}ACTION ")
        .map(|t| t.trim_end_matches('\u{1}'))
    {
        Some(t) => (t, true),
        None => (text, false),
    }
}

// IRC names and channels: no spaces, control characters or a leading colon, which
// would let them smuggle extra parameters or commands into a line.
pub fn valid_name(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && !s.starts_with(':')
        && !s.chars().any(|c| c.is_whitespace() || c.is_control())
}

// Strips line breaks and cuts the text to TEXT_MAX bytes on a character boundary.
pub fn clean_text(text: &str) -> String {
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

pub fn room_id(channel: &str) -> Option<i64> {
    channel.strip_prefix("#mp_")?.parse().ok()
}

pub fn room_channel(mp_id: i64) -> String {
    format!("#mp_{mp_id}")
}

// Usernames compare without case, and IRC writes spaces as underscores.
pub fn normalize_name(name: &str) -> String {
    name.trim().replace('_', " ").to_lowercase()
}

pub fn same_name(a: &str, b: &str) -> bool {
    normalize_name(a) == normalize_name(b)
}

// The name to log in or address someone with on IRC.
pub fn irc_name(username: &str) -> String {
    username.trim().replace(' ', "_")
}

// Connects and sends the login; Bancho answers with 001 if it takes it. Err when Bancho
// couldn't be reached at all, so the attempt never counted.
pub async fn connect(
    addr: &str,
    nick: &str,
    password: &str,
) -> Result<(BufReader<OwnedReadHalf>, OwnedWriteHalf), ()> {
    let stream = match timeout(LOGIN_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(stream)) => stream,
        Ok(Err(err)) => {
            tracing::warn!(error = %err, "could not reach Bancho");
            return Err(());
        }
        Err(_) => return Err(()),
    };
    let (read_half, mut writer) = stream.into_split();
    let login = format!("PASS {password}\r\nNICK {nick}\r\nUSER {nick} 0 * :{nick}\r\n");
    writer.write_all(login.as_bytes()).await.map_err(|_| ())?;
    Ok((BufReader::new(read_half), writer))
}

// Bancho mutes accounts that send too fast. At most `limit` lines go out in any
// `window`, and never two closer than `gap`.
pub struct Pacer {
    limit: usize,
    window: Duration,
    gap: Duration,
    sent: VecDeque<Instant>,
}

impl Pacer {
    pub fn new(limit: usize, window: Duration, gap: Duration) -> Self {
        Pacer {
            limit,
            window,
            gap,
            sent: VecDeque::new(),
        }
    }

    // When the next line may go out.
    pub fn next_send(&mut self, now: Instant) -> Instant {
        while self
            .sent
            .front()
            .is_some_and(|&t| now.duration_since(t) >= self.window)
        {
            self.sent.pop_front();
        }
        let by_gap = self.sent.back().map_or(now, |&t| t + self.gap);
        let by_window = if self.sent.len() < self.limit {
            now
        } else {
            self.sent[0] + self.window
        };
        by_gap.max(by_window).max(now)
    }

    pub fn record(&mut self, at: Instant) {
        self.sent.push_back(at);
    }

    // Lines sent in the last window, for the bots page.
    pub fn recent(&self, now: Instant) -> usize {
        self.sent
            .iter()
            .filter(|&&t| now.duration_since(t) < self.window)
            .count()
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
    fn paces_lines() {
        let start = Instant::now();
        let mut pacer = Pacer::new(3, Duration::from_secs(5), Duration::from_millis(400));
        assert_eq!(pacer.next_send(start), start);
        pacer.record(start);
        // The gap.
        assert_eq!(pacer.next_send(start), start + Duration::from_millis(400));
        pacer.record(start + Duration::from_millis(400));
        pacer.record(start + Duration::from_millis(800));
        // The window is full until the first line is 5 s old.
        let at = start + Duration::from_secs(1);
        assert_eq!(pacer.next_send(at), start + Duration::from_secs(5));
        assert_eq!(pacer.recent(at), 3);
    }
}
