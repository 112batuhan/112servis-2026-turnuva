//! What BanchoBot says in the room: maps starting and finishing, players leaving, and
//! what osu! says the map scored.

use crate::bancho::{
    room::{parse_left, MATCH_STARTED},
    tally::{score_message, standing_of, Standing},
};
use crate::db::autoref::LogLevel;

use super::{
    log, parse_roll, send, Action, AutoRef, Phase, Snapshot, WarmupMap, ABORT_WINDOW, SCORE_DELAY,
    SCORE_TRIES, START_COUNTDOWN, WARMUP_MAX_SECS,
};

const POOL_WARMUP: &str =
    "Havuzdaki mapler (ve diğer zorlukları) warmup olarak oynanamaz, başka bir map seçin.";

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
            // A pool map started by someone else (a referee in the room): the bot follows
            // it rather than open another over it.
            (Phase::Draft { .. } | Phase::Next, MATCH_STARTED) => self.started_elsewhere(s, out),
            (Phase::Waiting, MATCH_STARTED) if !s.bracket => self.started_elsewhere(s, out),
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
            (Phase::Warmup { .. }, "All players are ready") => self.warmup_ready(s, out),
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
    fn reopen(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
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

    // A pool map was started without the bot: it's followed as if the bot had opened it.
    fn started_elsewhere(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Some(entry) = s.room_entry() else {
            return;
        };
        log(
            out,
            LogLevel::Warn,
            format!(
                "{} bot dışında başlatıldı, bot onu takip ediyor.",
                entry.label
            ),
        );
        self.countdown = None;
        self.phase = Phase::Playing {
            map_id: entry.map_id,
            results: s.room.results.len(),
        };
    }

    // Why the warmup on can't be played: a pool song, or longer than 4 minutes.
    fn warmup_refusal(&self, s: &Snapshot, beatmap: i64) -> Option<String> {
        let known = self.warmups.get(&beatmap);
        if s.in_pool(beatmap, known.map(|w| w.beatmapset_id)) {
            return Some(POOL_WARMUP.into());
        }
        let secs = known.map(|w| w.seconds)?;
        (secs > WARMUP_MAX_SECS).then(|| {
            format!(
                "Bu map {}:{:02} uzunluğunda, warmup 4 dakikayı geçmemeli. Başka bir map seçin.",
                secs / 60,
                secs % 60
            )
        })
    }

    fn warmup_started(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if let Some(reason) = s.room.beatmap_id.and_then(|b| self.warmup_refusal(s, b)) {
            send(out, "!mp abort");
            send(out, reason);
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

    // Everyone is ready on the warmup the team chose: the bot starts it. Not before the
    // team chose one, nor a map the rules refuse, nor one osu! hasn't described yet.
    fn warmup_ready(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Phase::Warmup {
            played: false,
            beatmap: Some(_),
            starting: false,
            ..
        } = self.phase
        else {
            return;
        };
        let Some(beatmap) = s.room.beatmap_id else {
            return;
        };
        if s.room.playing || !s.everyone_present() {
            return;
        }
        if let Some(reason) = self.warmup_refusal(s, beatmap) {
            send(out, reason);
            return;
        }
        if !self.warmups.contains_key(&beatmap) {
            out.push(Action::WarmupMap(beatmap));
            return;
        }
        send(out, "Herkes hazır, warmup başlıyor.");
        send(out, format!("!mp start {START_COUNTDOWN}"));
        log(out, LogLevel::Info, "Herkes hazır, warmup başlatıldı.");
    }

    fn warmup_map(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Some(beatmap) = s.room.beatmap_id else {
            return;
        };
        match self.warmups.contains_key(&beatmap) {
            true => {
                if let Some(reason) = self.warmup_refusal(s, beatmap) {
                    send(out, reason);
                }
            }
            // A pool map is refused at once; the set and length come from osu!.
            false => {
                if s.in_pool(beatmap, None) {
                    send(out, POOL_WARMUP);
                }
                out.push(Action::WarmupMap(beatmap));
            }
        }
    }

    // osu! described a warmup beatmap.
    pub fn warmup_described(
        &mut self,
        s: &Snapshot,
        beatmap: i64,
        map: WarmupMap,
        out: &mut Vec<Action>,
    ) {
        let first = self.warmups.insert(beatmap, map).is_none();
        if !matches!(self.phase, Phase::Warmup { .. }) || s.room.beatmap_id != Some(beatmap) {
            return;
        }
        let pool_by_id = s.in_pool(beatmap, None);
        match self.warmup_refusal(s, beatmap) {
            // Already said for a pool map itself.
            Some(_) if first && pool_by_id => {}
            Some(reason) => {
                send(out, reason);
                if s.room.playing {
                    send(out, "!mp abort");
                }
            }
            // Everyone may have readied while osu! was asked.
            None if !s.room.playing
                && s.room
                    .slots
                    .iter()
                    .all(|slot| slot.status == crate::bancho::room::SlotStatus::Ready) =>
            {
                self.warmup_ready(s, out)
            }
            None => {}
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
