//! Coming back to a match after a restart, a bot that dropped, or a referee's manual
//! stretch: what happened meanwhile is read from the room and the match, and the bot goes
//! on from there rather than from where it stopped.

use uuid::Uuid;

use crate::bancho::tally::{standing_of, PoolEntry, Standing};
use crate::{db::autoref::LogLevel, draft::StepKind};

use super::draft::refusal;
use super::view::phase_name;
use super::{
    log, send, span, Action, AutoRef, CountdownKind, Phase, Snapshot, ROLL_TIME, WARMUP_TIME,
};

impl AutoRef {
    // Picks up again: the phase's timers start over, a pool map being played is followed,
    // a match the staff took past its start goes on from its next step, and a pick the
    // referee made (marked, or just put on in the room) is played.
    pub fn resume(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if self.phase == Phase::Done {
            return;
        }
        // A pool map being played, whatever the bot last did: it waits for it to finish.
        if s.room.playing {
            if let Some(entry) = s.room_entry() {
                let same =
                    matches!(self.phase, Phase::Playing { map_id, .. } if map_id == entry.map_id);
                if !same {
                    log(
                        out,
                        LogLevel::Info,
                        format!(
                            "Devam ediyor: {} oynanıyor, bitince sıradaki adıma geçilecek.",
                            entry.label
                        ),
                    );
                    self.countdown = None;
                    self.phase = Phase::Playing {
                        map_id: entry.map_id,
                        results: s.room.results.len(),
                    };
                }
                return;
            }
        }
        let before = matches!(
            self.phase,
            Phase::Waiting | Phase::Warmup { .. } | Phase::Roll { .. }
        );
        if before && s.started() {
            log(
                out,
                LogLevel::Info,
                "Devam ediyor: maç bot olmadan başlamış (ban, pick, roll ya da oynanan map var), \
                 sıradaki adımdan devam ediliyor.",
            );
            if !matches!(self.phase, Phase::Waiting) {
                send(out, "!mp clearhost");
            }
            self.countdown = None;
            self.phase = Phase::Next;
            return;
        }
        match self.phase.clone() {
            Phase::Waiting => {
                // A warmup the staff started before handing the match over.
                if s.bracket && s.room.playing {
                    let team = s.host_team().unwrap_or(0);
                    log(
                        out,
                        LogLevel::Info,
                        "Devam ediyor: bir warmup oynanıyor, bitince sıradaki warmup.",
                    );
                    self.phase = Phase::Warmup {
                        team,
                        until: s.now + WARMUP_TIME,
                        played: true,
                        skipped: [false, false],
                        beatmap: s.room.beatmap_id,
                        starting: false,
                    };
                }
            }
            Phase::Next | Phase::Done => {}
            Phase::Warmup { team, skipped, .. } => {
                self.resumed(out);
                if !s.room.playing {
                    self.enter_warmup(s, team, skipped, out);
                }
            }
            Phase::Roll { rolls, .. } => {
                self.resumed(out);
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
                self.stand_in_rolls(s, out);
            }
            // The turn is read again: a referee may have marked bans or picks meanwhile.
            Phase::Draft { kind, .. } => {
                self.resumed(out);
                self.phase = Phase::Next;
                if kind == StepKind::Pick {
                    self.take_room_pick(s, out);
                }
            }
            Phase::Ready { map_id, .. } => {
                self.resumed(out);
                self.open_again(s, map_id, out);
            }
            Phase::Playing { map_id, results } => {
                self.resumed(out);
                if s.room.results.len() > results {
                    self.phase = Phase::Scoring {
                        map_id,
                        fetch_at: s.now,
                        tries: 0,
                    };
                } else if !s.room.playing {
                    // Aborted by the referee meanwhile.
                    self.open_again(s, map_id, out);
                }
            }
            Phase::Scoring { map_id, tries, .. } => {
                self.resumed(out);
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now,
                    tries,
                };
            }
        }
    }

    // The bot stopped with a map on, or being played and aborted since. The referee may
    // have put another map on, taken the pick back or marked another meanwhile, so the map
    // isn't simply opened again: the one in the room is, if the match may play it now;
    // otherwise the match says what comes next (`next_step`), or, in a bracket, a pool map
    // the referee put on during a team's pick is that pick.
    fn open_again(&mut self, s: &Snapshot, was: Uuid, out: &mut Vec<Action>) {
        self.countdown = None;
        self.phase = Phase::Next;
        let on = s.room_entry();
        if let Some(entry) = on.filter(|e| playable_now(s, e)) {
            if entry.map_id != was {
                log(
                    out,
                    LogLevel::Info,
                    format!("Hakem odaya {} mapini açmış, o oynanacak.", entry.label),
                );
            }
            return self.enter_ready(s, entry, out);
        }
        // Left on from before, it's no sign of a pick.
        if on.is_some_and(|e| e.map_id != was) {
            self.take_room_pick(s, out);
        }
    }

    fn resumed(&self, out: &mut Vec<Action>) {
        log(
            out,
            LogLevel::Info,
            format!("Kaldığı yerden devam ediyor: {}.", phase_name(&self.phase)),
        );
    }

    // The bot stopped while a team was to pick, and the referee put a pool map on in the
    // room meanwhile without marking it: that's the team's pick. Only a map the team may
    // pick, and only when no marked pick waits to be played.
    fn take_room_pick(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Some(turn) = s.turn.filter(|t| t.kind == StepKind::Pick) else {
            return;
        };
        let Some(team_id) = turn.team_id else {
            return;
        };
        if s.picks.iter().any(|p| !s.finished.contains(&p.map_id)) {
            return;
        }
        let Some(entry) = s.room_entry() else {
            return;
        };
        if refusal(s, entry, StepKind::Pick, team_id).is_some() {
            return;
        }
        let name = s.team_name(team_id);
        send(
            out,
            format!(
                "{name} takımının pick'i hakemin açtığı {} olarak sayıldı.",
                entry.label
            ),
        );
        log(
            out,
            LogLevel::Info,
            format!(
                "Hakem {} mapini açmıştı: {name} takımının pick'i olarak işaretlendi.",
                entry.label
            ),
        );
        out.push(Action::Pick {
            map_id: entry.map_id,
            team_id,
        });
        self.enter_ready(s, entry, out);
    }
}

// Whether the match may play this pool map now: a lobby's unplayed maps (not the
// tiebreaker); in a bracket, a pick not yet played, or the tiebreaker once the score
// calls for it (`next_step`).
fn playable_now(s: &Snapshot, entry: &PoolEntry) -> bool {
    if !s.bracket {
        return !entry.tiebreaker && !s.played.contains(&entry.map_id);
    }
    if s.finished.contains(&entry.map_id) {
        return false;
    }
    let standing = match (s.best_of, s.score) {
        (Some(best_of), Some((a, b))) => standing_of(best_of, a, b),
        _ => Standing::Open,
    };
    match standing {
        Standing::Tiebreaker => entry.tiebreaker,
        Standing::Open | Standing::MatchPoint => {
            !entry.tiebreaker && s.picks.iter().any(|p| p.map_id == entry.map_id)
        }
        Standing::Won | Standing::Over => false,
    }
}
