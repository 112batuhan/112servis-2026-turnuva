//! Coming back to a match after a restart or a referee's manual stretch.

use crate::db::autoref::LogLevel;

use super::view::phase_name;
use super::{log, span, Action, AutoRef, CountdownKind, Phase, Snapshot, ROLL_TIME};

impl AutoRef {
    // Picks up again after the match was in manual mode (a referee's switch or a
    // -durdur) or a restart: the phase's timers start over, and whatever happened in
    // between is read from the room.
    pub fn resume(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        log(
            out,
            LogLevel::Info,
            format!("Kaldığı yerden devam ediyor: {}.", phase_name(&self.phase)),
        );
        match self.phase.clone() {
            Phase::Waiting | Phase::Next | Phase::Done => {}
            Phase::Warmup { team, skipped, .. } => {
                if !s.room.playing {
                    self.enter_warmup(s, team, skipped, out);
                }
            }
            Phase::Roll { rolls, .. } => {
                // The rolls already in stand.
                self.phase = Phase::Roll {
                    until: s.now + ROLL_TIME,
                    rolls,
                };
                let secs = ROLL_TIME.whole_seconds();
                self.start_countdown(
                    s,
                    CountdownKind::Roll,
                    secs,
                    format!(
                        "Pick ve ban sırasını belirlemek için !roll komutunu kullanın, {} süreniz var.",
                        span(secs)
                    ),
                    out,
                );
            }
            // The turn is read again: a referee may have marked bans or picks meanwhile.
            Phase::Draft { .. } => self.phase = Phase::Next,
            Phase::Ready { map_id, .. } => {
                if s.room.playing {
                    self.phase = Phase::Playing {
                        map_id,
                        results: s.room.results.len(),
                    };
                } else if s.played.contains(&map_id) {
                    self.phase = Phase::Next;
                } else if let Some(entry) = s.entry(map_id) {
                    self.enter_ready(s, entry, out);
                } else {
                    self.phase = Phase::Next;
                }
            }
            Phase::Playing { map_id, results } if !s.room.playing => {
                if s.room.results.len() > results {
                    self.phase = Phase::Scoring {
                        map_id,
                        fetch_at: s.now,
                        tries: 0,
                    };
                } else {
                    self.reopen(s, out);
                }
            }
            Phase::Playing { .. } => {}
            Phase::Scoring { map_id, tries, .. } => {
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now,
                    tries,
                };
            }
        }
    }
}
