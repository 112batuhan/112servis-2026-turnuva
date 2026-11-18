//! What BanchoBot says in the room: maps starting and finishing, players leaving, and
//! what osu! says the map scored.

use crate::bancho::{
    room::{parse_left, MATCH_STARTED},
    tally::{score_message, standing_of, Standing},
};
use crate::db::autoref::LogLevel;

use super::{
    log, parse_roll, send, Action, AutoRef, Phase, Snapshot, ABORT_WINDOW, SCORE_DELAY,
    SCORE_TRIES, WARMUP_MAX_SECS,
};

impl AutoRef {
    // A line BanchoBot said in the room, once the room's state has it.
    pub fn bancho(&mut self, s: &Snapshot, text: &str, out: &mut Vec<Action>) {
        if let Some((name, n)) = parse_roll(text) {
            return self.take_roll(s, name, n, out);
        }
        match (self.phase.clone(), text) {
            (Phase::Ready { .. }, "All players are ready") => self.try_start(s, false, out),
            (Phase::Ready { map_id, .. }, MATCH_STARTED) => {
                self.phase = Phase::Playing {
                    map_id,
                    results: s.room.results.len(),
                };
            }
            (Phase::Playing { .. }, "Aborted the match") => self.reopen(s, out),
            (Phase::Playing { map_id, .. }, "The match has finished!") => {
                log(
                    out,
                    LogLevel::Info,
                    "Map bitti, osu!'dan skorlar bekleniyor.",
                );
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now + SCORE_DELAY,
                    tries: 0,
                };
            }
            (Phase::Playing { .. }, _) => {
                if let Some(name) = parse_left(text) {
                    self.dropped(s, name, out);
                }
            }
            (Phase::Warmup { .. }, MATCH_STARTED) => self.warmup_started(s, out),
            (Phase::Warmup { team, .. }, "The match has finished!") => {
                self.next_warmup(s, team, out)
            }
            (Phase::Warmup { .. }, _)
                if text.starts_with("Changed beatmap to ")
                    || text.starts_with("Beatmap changed to: ") =>
            {
                self.warmup_map(s, out)
            }
            _ => {}
        }
    }

    // The map that was aborted goes back on.
    pub(super) fn reopen(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Phase::Playing { map_id, .. } = self.phase.clone() else {
            return;
        };
        match s.entry(map_id) {
            Some(entry) => self.enter_ready(s, entry, out),
            None => self.phase = Phase::Next,
        }
    }

    // Someone left while a map was played: within its first 30 seconds, it's aborted on
    // their team's account (in a qualifier only on its first map; after that it plays on
    // without them).
    fn dropped(&mut self, s: &Snapshot, name: &str, out: &mut Vec<Action>) {
        let Some(p) = s.player(name) else {
            return;
        };
        let early = s
            .room
            .map_started_at
            .is_some_and(|t| s.now - t <= ABORT_WINDOW);
        if !early {
            return;
        }
        if !s.bracket && !s.played.is_empty() {
            log(
                out,
                LogLevel::Info,
                format!("{} düştü, maç onsuz devam ediyor.", p.username),
            );
            return;
        }
        let team_id = p.team_id;
        if self.aborts_left(team_id) == 0 {
            log(
                out,
                LogLevel::Info,
                format!("{} düştü ama takımının abort hakkı kalmadı.", p.username),
            );
            return;
        }
        self.uses_mut(team_id).aborts += 1;
        let left = self.aborts_left(team_id);
        send(out, "!mp abort");
        send(
            out,
            format!(
                "{} oyundan düştü. Abort kullanıldı. {left} hakkınız kaldı.",
                p.username
            ),
        );
        log(
            out,
            LogLevel::Info,
            format!(
                "{} düştüğü için map iptal edildi ({} hakkı kaldı).",
                p.username, left
            ),
        );
    }

    fn warmup_started(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let beatmap = s.room.beatmap_id;
        let in_pool = beatmap.is_some_and(|b| s.entries.iter().any(|e| e.beatmap_id == b));
        let too_long = beatmap
            .and_then(|b| self.lengths.get(&b))
            .is_some_and(|&secs| secs > WARMUP_MAX_SECS);
        if in_pool || too_long {
            send(out, "!mp abort");
            send(
                out,
                if in_pool {
                    "Havuzdaki mapler warmup olarak oynanamaz, başka bir map seçin."
                } else {
                    "Bu map 4 dakikadan uzun, başka bir warmup seçin."
                },
            );
            log(
                out,
                LogLevel::Warn,
                "Kurallara uymayan warmup iptal edildi.",
            );
            return;
        }
        if let Phase::Warmup { played, .. } = &mut self.phase {
            *played = true;
        }
    }

    fn warmup_map(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Some(beatmap) = s.room.beatmap_id else {
            return;
        };
        if s.entries.iter().any(|e| e.beatmap_id == beatmap) {
            send(
                out,
                "Havuzdaki mapler warmup olarak oynanamaz, başka bir map seçin.",
            );
            return;
        }
        match self.lengths.get(&beatmap) {
            Some(&secs) => self.warmup_too_long(s, secs, out),
            None => out.push(Action::WarmupLength(beatmap)),
        }
    }

    // osu! said how long a warmup beatmap is.
    pub fn warmup_length(&mut self, s: &Snapshot, beatmap: i64, secs: i32, out: &mut Vec<Action>) {
        self.lengths.insert(beatmap, secs);
        if matches!(self.phase, Phase::Warmup { .. }) && s.room.beatmap_id == Some(beatmap) {
            self.warmup_too_long(s, secs, out);
        }
    }

    fn warmup_too_long(&mut self, s: &Snapshot, secs: i32, out: &mut Vec<Action>) {
        if secs <= WARMUP_MAX_SECS {
            return;
        }
        send(
            out,
            format!(
                "Bu map {}:{:02} uzunluğunda, warmup 4 dakikayı geçmemeli. Başka bir map seçin.",
                secs / 60,
                secs % 60
            ),
        );
        if s.room.playing {
            send(out, "!mp abort");
        }
    }

    // osu!'s record of the rooms was read: `confirmed` when it has the map that just
    // finished.
    pub fn osu_read(&mut self, s: &Snapshot, confirmed: bool, out: &mut Vec<Action>) {
        let Phase::Scoring { map_id, tries, .. } = self.phase.clone() else {
            return;
        };
        if !confirmed {
            if tries + 1 < SCORE_TRIES {
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: s.now + SCORE_DELAY,
                    tries: tries + 1,
                };
                return;
            }
            log(
                out,
                LogLevel::Warn,
                "osu! skorları getiremedi, sohbetteki skorlarla devam ediliyor.",
            );
        } else {
            log(out, LogLevel::Info, "Skorlar osu!'dan doğrulandı.");
        }
        // The map that decided the match is announced with the winner instead.
        let decided = matches!(
            (s.best_of, s.score),
            (Some(bo), Some((a, b))) if standing_of(bo, a, b) == Standing::Won
        );
        if let (true, false, Some((a, b))) = (s.bracket, decided, s.score) {
            if s.teams.len() == 2 {
                let names = [s.teams[0].name.as_str(), s.teams[1].name.as_str()];
                send(out, score_message(names, [a, b], None));
            }
        }
        self.phase = Phase::Next;
    }
}
