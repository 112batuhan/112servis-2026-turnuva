//! The bot's own countdowns: the no-show wait, a team's warmup turn, the roll and a map's
//! ready timer (Bancho's !mp timer isn't used). The room is told how long it has when the
//! wait starts, then at every whole minute left, at 30 and at 10 seconds, and when it's
//! over.
//!
//! What the room is told is always true from the moment the line goes out: a reminder
//! held up in the bot's queue moves the end back by as long as it was late (`said`, fed
//! the time each of the bot's lines was really sent). The bot acts `GRACE` after the end
//! it announced, so a slow line or a slow client costs nobody the step.

use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};

use super::{send, Action, AutoRef, Phase, Snapshot};

// How long after the end the room was told the bot waits before it acts.
pub const GRACE: Duration = Duration::seconds(10);
// A reminder this late or less isn't worth moving the end for.
const LATE: Duration = Duration::seconds(1);

// What a countdown is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CountdownKind {
    // The missing players' ten minutes, from the match's time.
    NoShow,
    // A team's turn to open a warmup.
    Warmup,
    Roll,
    // A map's ready timer, and the NoFail grace after it.
    Ready,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Countdown {
    pub kind: CountdownKind,
    // The end the room was told; the bot acts GRACE after it.
    #[serde(with = "time::serde::rfc3339")]
    pub ends: OffsetDateTime,
    // The seconds left the room was last told (at the start: the whole wait).
    pub told: i64,
    // That line, until the bot is seen sending it.
    #[serde(default)]
    pub pending: Option<String>,
}

impl Countdown {
    // When the bot acts.
    pub fn deadline(&self) -> OffsetDateTime {
        self.ends + GRACE
    }
}

// "2 dakika", "1 dakika 30 saniye", "10 saniye".
pub fn span(secs: i64) -> String {
    let (m, s) = (secs / 60, secs % 60);
    match (m, s) {
        (0, s) => format!("{s} saniye"),
        (m, 0) => format!("{m} dakika"),
        (m, s) => format!("{m} dakika {s} saniye"),
    }
}

// The reminders of a wait of `total` seconds, in seconds left, latest first: each whole
// minute, 30 and 10, all shorter than the wait itself.
fn marks(total: i64) -> Vec<i64> {
    let mut marks: Vec<i64> = (1..=total / 60).map(|m| m * 60).collect();
    marks.extend([30, 10]);
    marks.retain(|&m| m < total);
    marks.sort_unstable_by(|a, b| b.cmp(a));
    marks.dedup();
    marks
}

impl AutoRef {
    // Starts a wait of `secs`, announced by `opening` (which says how long it is). The
    // phase is set first; its deadline is set here.
    pub(super) fn start_countdown(
        &mut self,
        s: &Snapshot,
        kind: CountdownKind,
        secs: i64,
        opening: String,
        out: &mut Vec<Action>,
    ) {
        self.countdown = Some(Countdown {
            kind,
            ends: s.now + Duration::seconds(secs),
            told: secs,
            pending: Some(opening.clone()),
        });
        self.sync_deadline();
        send(out, opening);
    }

    // The deadline of the countdown, if one runs for this phase.
    pub fn countdown_deadline(&self, kind: CountdownKind) -> Option<OffsetDateTime> {
        self.countdown
            .as_ref()
            .filter(|c| c.kind == kind)
            .map(Countdown::deadline)
    }

    // The next reminder, once it's due. Run on every tick, before the phase's own clock.
    pub(super) fn remind(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Some(c) = self.countdown.clone() else {
            return;
        };
        let matches = matches!(
            (c.kind, &self.phase),
            (CountdownKind::NoShow, Phase::Waiting)
                | (CountdownKind::Warmup, Phase::Warmup { .. })
                | (CountdownKind::Roll, Phase::Roll { .. })
                | (CountdownKind::Ready, Phase::Ready { .. })
        );
        if !matches {
            self.countdown = None;
            return;
        }
        // Held while there's nothing to count down to: a warmup being played, a map
        // starting or its mods being read.
        let counting = match &self.phase {
            Phase::Warmup { played, .. } => !played && !s.room.playing,
            Phase::Ready {
                starting, check_at, ..
            } => starting.is_none() && check_at.is_none(),
            _ => true,
        };
        if !counting {
            return;
        }
        let left = c.ends - s.now;
        let Some(mark) = marks(c.told)
            .into_iter()
            .filter(|&m| left <= Duration::seconds(m))
            .min()
        else {
            return;
        };
        let Some(text) = self.reminder(s, c.kind, mark) else {
            return;
        };
        let mut c = c;
        // Reached late (a pause, or the tick): the end moves so what's said is true.
        if left < Duration::seconds(mark) - LATE {
            c.ends = s.now + Duration::seconds(mark);
        }
        c.told = mark;
        c.pending = Some(text.clone());
        self.countdown = Some(c);
        self.sync_deadline();
        send(out, text);
    }

    fn reminder(&self, s: &Snapshot, kind: CountdownKind, secs: i64) -> Option<String> {
        let left = span(secs);
        Some(match kind {
            CountdownKind::NoShow => format!("Eksik oyuncular için {left} kaldı."),
            CountdownKind::Warmup => {
                let Phase::Warmup { team, .. } = &self.phase else {
                    return None;
                };
                let name = s.teams.get(*team).map_or("?", |t| t.name.as_str());
                format!("{name}: warmup açmak için {left} kaldı.")
            }
            CountdownKind::Roll => format!("!roll için {left} kaldı."),
            CountdownKind::Ready => format!("Map {left} sonra başlayacak."),
        })
    }

    // One of the bot's own lines went out at `at`. A countdown line that went out late
    // moves the end back by as long, so what it said holds from when the room saw it.
    pub fn said(&mut self, text: &str, at: OffsetDateTime) {
        let Some(c) = self.countdown.as_mut() else {
            return;
        };
        if c.pending.as_deref() != Some(text) {
            return;
        }
        c.pending = None;
        let due = c.ends - Duration::seconds(c.told);
        if at - due > LATE {
            c.ends = at + Duration::seconds(c.told);
            self.sync_deadline();
        }
    }

    // The phase's deadline follows the countdown's.
    fn sync_deadline(&mut self) {
        let Some(deadline) = self.countdown.as_ref().map(Countdown::deadline) else {
            return;
        };
        match &mut self.phase {
            Phase::Warmup { until, .. }
            | Phase::Roll { until, .. }
            | Phase::Ready { until, .. } => *until = deadline,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reminders_every_minute_then_30_and_10() {
        assert_eq!(marks(600)[..2], [540, 480]);
        assert_eq!(marks(180), [120, 60, 30, 10]);
        assert_eq!(marks(120), [60, 30, 10]);
        assert_eq!(marks(30), [10]);
        assert_eq!(marks(10), Vec::<i64>::new());
        assert_eq!(span(120), "2 dakika");
        assert_eq!(span(90), "1 dakika 30 saniye");
        assert_eq!(span(10), "10 saniye");
    }
}
