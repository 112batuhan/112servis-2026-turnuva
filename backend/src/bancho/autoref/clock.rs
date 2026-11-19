//! The clock: waiting for the players, the no-show forfeit, and every phase's timer.

use time::Duration;

use crate::bancho::{
    context::MatchTeam, names::same_name, room::commands::player_ref, room::SlotStatus,
    tally::RosterPlayer,
};
use crate::db::autoref::LogLevel;

use super::{
    log, send, span, Action, AutoRef, CountdownKind, Phase, Snapshot, GRACE, LOCK_EVERY,
    NO_SHOW_WAIT, START_COUNTDOWN, START_TIMEOUT, WARMUP_PLAY_MAX,
};

impl AutoRef {
    pub fn tick(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let now = s.now;
        self.keep_locked(s, out);
        // The turn changed under it (a referee marked, took back or chose the next team):
        // it asks again, with a new timer, before the old one's reminders.
        if let Phase::Draft { team_id, kind, .. } = self.phase {
            if s.turn
                .is_some_and(|t| t.team_id != Some(team_id) || t.kind != kind)
            {
                log(
                    out,
                    LogLevel::Info,
                    "Ban/pick sırası değişti, yeniden soruluyor.",
                );
                self.next_step(s, out);
            }
        }
        self.remind(s, out);
        match self.phase.clone() {
            Phase::Waiting => self.waiting(s, out),
            Phase::Warmup {
                team,
                until,
                played,
                beatmap,
                starting,
                ..
            } => {
                let started = s.room.map_started_at;
                if s.room.playing && started.is_some_and(|t| now - t > WARMUP_PLAY_MAX) {
                    send(out, "!mp abort");
                    send(out, "Warmup 4 dakikayı geçti, sonlandırıldı.");
                    log(
                        out,
                        LogLevel::Warn,
                        "Warmup 4,5 dakikayı geçtiği için iptal edildi.",
                    );
                    self.next_warmup(s, team, out);
                } else if !played && !s.room.playing && now >= until {
                    let name = s.teams.get(team).map_or("?", |t| t.name.as_str());
                    if beatmap.is_some() && !starting {
                        // The chosen warmup is on: it starts whoever is ready.
                        send(out, "Süre doldu, warmup başlıyor.");
                        send(out, format!("!mp start {START_COUNTDOWN}"));
                        log(out, LogLevel::Info, "Warmup süresi doldu, başlatıldı.");
                        if let Phase::Warmup {
                            until, starting, ..
                        } = &mut self.phase
                        {
                            *until = now + START_TIMEOUT;
                            *starting = true;
                        }
                    } else if starting {
                        log(
                            out,
                            LogLevel::Warn,
                            "Warmup başlamadı, sıradaki adıma geçiliyor.",
                        );
                        self.next_warmup(s, team, out);
                    } else {
                        send(out, format!("{name} takımının warmup süresi doldu."));
                        log(
                            out,
                            LogLevel::Info,
                            format!("{name} warmup seçmedi, süre doldu."),
                        );
                        self.next_warmup(s, team, out);
                    }
                }
            }
            Phase::Roll { until, .. } if now >= until => {
                send(out, "!roll süresi doldu.");
                self.resolve_roll(s, out);
            }
            Phase::Draft {
                team_id,
                kind,
                until,
                ..
            } if now >= until => self.missed(s, team_id, kind, out),
            Phase::Ready {
                until,
                nf_warned,
                fm_warned,
                check_at,
                forced,
                starting,
                ..
            } => {
                if let Some(at) = starting {
                    // The countdown went out but the map never started (a line lost, or a
                    // referee's !mp aborttimer): ask again.
                    if !s.room.playing && now - at > START_TIMEOUT {
                        log(out, LogLevel::Warn, "Map başlamadı, tekrar deneniyor.");
                        self.set_ready(|r| r.starting = None);
                        self.try_start(s, true, out);
                    }
                } else if let Some(at) = check_at {
                    if now >= at {
                        self.check_mods(s, forced, out);
                    }
                } else if now >= until {
                    // After the mods grace, check_mods says so itself.
                    if !nf_warned && !fm_warned {
                        send(out, "Süre doldu, map başlıyor.");
                    }
                    self.try_start(s, true, out);
                }
            }
            Phase::Scoring {
                map_id,
                fetch_at,
                tries,
            } if now >= fetch_at => {
                out.push(Action::FetchOsu);
                // Asked again if the answer never comes.
                self.phase = Phase::Scoring {
                    map_id,
                    fetch_at: now + Duration::seconds(30),
                    tries,
                };
            }
            Phase::Next => self.next_step(s, out),
            _ => {}
        }
    }

    // The room stays locked while the bot runs it, so players can't change their slot or
    // colour themselves; Bancho is asked again while it says otherwise.
    fn keep_locked(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if self.phase == Phase::Done || s.room.closed {
            return;
        }
        if s.room.locked == Some(true) {
            self.lock_asked = None;
            return;
        }
        if self.lock_asked.is_some_and(|t| s.now - t < LOCK_EVERY) {
            return;
        }
        if s.room.locked == Some(false) {
            log(
                out,
                LogLevel::Warn,
                "Odanın kilidi açılmış, tekrar kilitleniyor.",
            );
        }
        self.lock_asked = Some(s.now);
        send(out, "!mp lock");
    }

    // Before the match: once everyone is in (at its time, or earlier if they're all
    // ready), it begins; at its time the missing are invited again; ten minutes after,
    // whoever isn't complete is out.
    fn waiting(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if s.roster.is_empty() {
            return;
        }
        let now = s.now;
        let due = s.starts_at.is_none_or(|t| now >= t);
        if s.everyone_present() {
            let ready = s.room.slots.iter().all(|slot| {
                slot.status == SlotStatus::Ready
                    || !s.roster.iter().any(|p| {
                        slot.user_id == Some(p.osu_id) || same_name(&slot.username, &p.username)
                    })
            });
            if due || ready {
                self.begin(s, out);
            }
            return;
        }
        if s.starts_at.is_some() && due && !self.called {
            self.called = true;
            let missing: Vec<&RosterPlayer> = s.roster.iter().filter(|p| !s.present(p)).collect();
            for p in &missing {
                send(out, format!("!mp invite {}", player_ref(p.osu_id)));
            }
            // Ten minutes from the time, or from the room if it was made later.
            let secs = s
                .deadline()
                .map_or(0, |d| (d - now).whole_seconds())
                .max(NO_SHOW_WAIT.whole_seconds());
            self.start_countdown(
                s,
                CountdownKind::NoShow,
                secs,
                if s.bracket {
                    format!(
                        "Maç saati geldi. Eksik oyuncular için {} bekleniyor, gelmeyen takım \
                         kaybetmiş sayılacak.",
                        span(secs)
                    )
                } else {
                    format!(
                        "Maç saati geldi. Eksik oyuncular için {} bekleniyor, gelmeyen takımlar \
                         lobiden çıkarılacak.",
                        span(secs)
                    )
                },
                out,
            );
            let names: Vec<&str> = missing.iter().map(|p| p.username.as_str()).collect();
            log(
                out,
                LogLevel::Info,
                format!(
                    "Maç saati geldi, eksikler tekrar davet edildi: {}",
                    names.join(", ")
                ),
            );
        }
        let deadline = self
            .countdown_deadline(CountdownKind::NoShow)
            .or_else(|| s.deadline().map(|d| d + GRACE));
        if deadline.is_some_and(|d| now >= d) {
            self.no_show(s, out);
        }
    }

    fn no_show(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let complete: Vec<bool> = s.teams.iter().map(|t| s.complete(t.id)).collect();
        if !complete.iter().any(|&c| c) {
            out.push(Action::Handover(format!(
                "{} beklendi ama hiçbir takım tam kadro gelmedi.",
                span(NO_SHOW_WAIT.whole_seconds())
            )));
            return;
        }
        if s.bracket {
            let [a, b] = [&s.teams[0], &s.teams[1]];
            let (slot, winner, loser) = if complete[0] { (2, a, b) } else { (1, b, a) };
            send(
                out,
                format!(
                    "{} takımı {} içinde tam kadro gelmedi. Maç hükmen {} takımına verildi.",
                    loser.name,
                    span(NO_SHOW_WAIT.whole_seconds()),
                    winner.name
                ),
            );
            log(
                out,
                LogLevel::Warn,
                format!("{} gelmedi: hükmen {} kazandı.", loser.name, winner.name),
            );
            out.push(Action::Forfeit(slot));
            self.phase = Phase::Done;
            return;
        }
        let gone: Vec<&MatchTeam> = s
            .teams
            .iter()
            .zip(&complete)
            .filter(|(_, c)| !**c)
            .map(|(t, _)| t)
            .collect();
        for team in &gone {
            for p in s.members(team.id).filter(|p| s.present(p)) {
                send(out, format!("!mp kick {}", player_ref(p.osu_id)));
            }
        }
        let names: Vec<&str> = gone.iter().map(|t| t.name.as_str()).collect();
        send(
            out,
            format!(
                "{} tam kadro gelmediği için lobiden çıkarıldı.",
                names.join(", ")
            ),
        );
        log(
            out,
            LogLevel::Warn,
            format!(
                "Tam kadro gelmeyen takımlar lobiden çıkarıldı: {}",
                names.join(", ")
            ),
        );
        out.push(Action::RemoveTeams(gone.iter().map(|t| t.id).collect()));
        self.begin(s, out);
    }

    fn begin(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        log(out, LogLevel::Info, "Herkes odada, maç başlıyor.");
        if s.bracket {
            self.enter_warmup(s, 0, [false, false], out);
        } else {
            send(
                out,
                "Herkes burada, maç başlıyor! Mapler sırayla açılacak, iyi şanslar.",
            );
            self.next_step(s, out);
        }
    }
}
