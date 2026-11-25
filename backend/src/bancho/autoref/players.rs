//! What the players do: their commands in the room (!rapor, !panik, !abort), and on the
//! match's room page those three plus their bans, picks and warmups, which only the page
//! takes.

use uuid::Uuid;

use crate::bancho::tally::RosterPlayer;
use crate::{db::autoref::LogLevel, draft::StepKind, error::PlayerError};

use super::draft::refusal;
use super::{
    command, log, send, span, Action, AutoRef, Command, CountdownKind, Phase, Snapshot, WarmupMap,
    ABORT_WINDOW, REPORT_EVERY, REPORT_MAX, WARMUP_MAX_SECS,
};
use crate::bancho::room::commands::{map_command, mods_command};

// A warmup chosen with less than this left on its team's turn gets this long instead, so
// everyone has time to download it before it's started.
const WARMUP_LEAD: i64 = 60;

// What a player did on the match's room page.
#[derive(Debug, Clone)]
pub enum Web {
    // Their team's ban or pick, held for three seconds on the page.
    Draft(Uuid),
    // They began holding that map: the turn waits for the hold (HOLD_WAIT).
    Hold(Uuid),
    // Their team's warmup, as osu! described it.
    Warmup(i64, WarmupMap),
    // Their team won't play a warmup (now, or when its turn comes).
    SkipWarmup,
    Abort,
    Stop,
    Report(String),
}

impl AutoRef {
    // A line a player wrote in the room. `enabled`: the match is in auto mode; !rapor
    // and !panik work either way. Bans, picks and warmups aren't taken here: the room
    // page takes them (`web`).
    pub fn player(
        &mut self,
        s: &Snapshot,
        name: &str,
        text: &str,
        enabled: bool,
        out: &mut Vec<Action>,
    ) {
        let Some(p) = s.player(name).cloned() else {
            return;
        };
        let Some((command, rest)) = command(text) else {
            return;
        };
        match command {
            Command::Report => {
                if let Err(err) = self.report(s, &p, rest, out) {
                    let line = match err {
                        PlayerError::ReportEmpty => "kullanım: !rapor <mesajınız>",
                        _ => "bir dakikada bir rapor gönderebilirsiniz.",
                    };
                    send(out, format!("{}: {line}", p.username));
                }
            }
            // Pressed again within the minute, it's quietly ignored.
            Command::Stop => {
                let _ = self.stop(s, &p, out);
            }
            Command::Abort if enabled && self.phase != Phase::Done => {
                if let Err(err) = self.abort(s, &p, out) {
                    send(out, abort_refusal(err, &p.username));
                }
            }
            Command::Abort | Command::Invite => {}
        }
    }

    // What a player did on the room page: the same as in the room, its refusal coming
    // back to the page rather than said in the room.
    pub fn web(
        &mut self,
        s: &Snapshot,
        user_id: Uuid,
        action: Web,
        enabled: bool,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        let p = s
            .roster
            .iter()
            .find(|p| p.user_id == user_id)
            .cloned()
            .ok_or(PlayerError::NotInMatch)?;
        match action {
            Web::Report(text) => return self.report(s, &p, text.trim(), out),
            Web::Stop => return self.stop(s, &p, out),
            _ => {}
        }
        if !enabled || self.phase == Phase::Done {
            return Err(PlayerError::Manual);
        }
        match action {
            Web::Abort => self.abort(s, &p, out),
            Web::Draft(map_id) => self.draft(s, &p, map_id, out),
            Web::Hold(map_id) => self.hold(s, &p, map_id),
            Web::Warmup(beatmap, map) => self.choose_warmup(s, &p, beatmap, map, out),
            Web::SkipWarmup => self.skip_warmup(s, &p, out),
            Web::Report(_) | Web::Stop => Ok(()),
        }
    }

    fn report(
        &mut self,
        s: &Snapshot,
        p: &RosterPlayer,
        text: &str,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        if text.is_empty() {
            return Err(PlayerError::ReportEmpty);
        }
        if self
            .reports
            .get(&p.osu_id)
            .is_some_and(|&t| s.now - t < REPORT_EVERY)
        {
            return Err(PlayerError::TooSoon);
        }
        self.reports.insert(p.osu_id, s.now);
        let text: String = text.chars().take(REPORT_MAX).collect();
        send(
            out,
            format!("{}: raporun hostlara ve hakemlere iletildi.", p.username),
        );
        log(
            out,
            LogLevel::Warn,
            format!("Rapor ({}): {text}", p.username),
        );
        out.push(Action::Report {
            user_id: p.user_id,
            team_id: p.team_id,
            text,
        });
        Ok(())
    }

    // !panik, for emergencies, any number of times: the bot stops (the match goes to
    // manual until a referee switches it back) and the staff hear it. Once a minute per
    // player, so it can't flood them.
    fn stop(
        &mut self,
        s: &Snapshot,
        p: &RosterPlayer,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        if self
            .stops
            .get(&p.osu_id)
            .is_some_and(|&t| s.now - t < REPORT_EVERY)
        {
            return Err(PlayerError::TooSoon);
        }
        self.stops.insert(p.osu_id, s.now);
        // A countdown or a mods check waiting is dropped; it's set again on resuming.
        self.set_ready(|r| {
            r.check_at = None;
            r.starting = None;
        });
        send(
            out,
            "Panik komutu kullanıldı. Bu komut sadece acil durumlarda kullanılmalıdır. Hostlar \
             bilgilendirildi, eğer bu komut kötü niyetle kullanıldıysa !rapor komutuyla bildiriniz.",
        );
        out.push(Action::Stop {
            user_id: p.user_id,
            team_id: p.team_id,
            username: p.username.clone(),
        });
        Ok(())
    }

    fn abort(
        &mut self,
        s: &Snapshot,
        p: &RosterPlayer,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        if !matches!(self.phase, Phase::Playing { .. }) || !s.room.playing {
            return Err(PlayerError::NoMapPlaying);
        }
        let early = s
            .room
            .map_started_at
            .is_some_and(|t| s.now - t <= ABORT_WINDOW);
        if !early {
            return Err(PlayerError::AbortTooLate);
        }
        if self.aborts_left(p.team_id) == 0 {
            return Err(PlayerError::NoAbortsLeft);
        }
        self.uses_mut(p.team_id).aborts += 1;
        let left = self.aborts_left(p.team_id);
        send(out, "!mp abort");
        send(
            out,
            format!("{} abort kullandı. {left} hakkınız kaldı.", p.username),
        );
        log(
            out,
            LogLevel::Info,
            format!("{} abort kullandı ({left} hakkı kaldı).", p.username),
        );
        Ok(())
    }

    // The team's ban or pick, from the page: held for three seconds there, so it isn't
    // asked again.
    fn draft(
        &mut self,
        s: &Snapshot,
        p: &RosterPlayer,
        map_id: Uuid,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        let Phase::Draft { team_id, kind, .. } = self.phase.clone() else {
            return Err(PlayerError::NotYourTurn);
        };
        if p.team_id != team_id {
            return Err(PlayerError::NotYourTurn);
        }
        let entry = s.entry(map_id).cloned().ok_or(PlayerError::NotYourTurn)?;
        if let Some(reason) = refusal(s, &entry, kind, team_id) {
            return Err(reason);
        }
        let name = s.team_name(team_id);
        let did = match kind {
            StepKind::Ban => "banladı",
            StepKind::Pick => "seçti",
        };
        log(
            out,
            LogLevel::Info,
            format!("{name} {} {did} ({}).", entry.label, p.username),
        );
        match kind {
            StepKind::Ban => {
                send(
                    out,
                    format!("{name} takımı {} mapini banladı.", entry.label),
                );
                out.push(Action::Ban {
                    map_id: entry.map_id,
                    team_id,
                });
                self.phase = Phase::Next;
            }
            StepKind::Pick => {
                send(out, format!("{name} takımı {} mapini seçti.", entry.label));
                out.push(Action::Pick {
                    map_id: entry.map_id,
                    team_id,
                });
                self.enter_ready(s, &entry, out);
            }
        }
        Ok(())
    }

    // A player began holding a map the team may take: until HOLD_WAIT has passed, the
    // turn doesn't run out (the hold's ban or pick, three seconds later, is taken even
    // past its time).
    fn hold(&mut self, s: &Snapshot, p: &RosterPlayer, map_id: Uuid) -> Result<(), PlayerError> {
        let Phase::Draft { team_id, kind, .. } = self.phase else {
            return Err(PlayerError::NotYourTurn);
        };
        if p.team_id != team_id {
            return Err(PlayerError::NotYourTurn);
        }
        let entry = s.entry(map_id).ok_or(PlayerError::NotYourTurn)?;
        if let Some(reason) = refusal(s, entry, kind, team_id) {
            return Err(reason);
        }
        self.held = Some((team_id, s.now));
        Ok(())
    }

    // Whether the player's team may choose its warmup now: asked before osu! is asked
    // to describe the beatmap.
    pub fn check_warmup(&self, s: &Snapshot, user_id: Uuid) -> Result<(), PlayerError> {
        let p = s
            .roster
            .iter()
            .find(|p| p.user_id == user_id)
            .ok_or(PlayerError::NotInMatch)?;
        let Phase::Warmup { team, played, .. } = self.phase else {
            return Err(PlayerError::NotYourWarmup);
        };
        if s.teams.get(team).map(|t| t.id) != Some(p.team_id) {
            return Err(PlayerError::NotYourWarmup);
        }
        if played || s.room.playing {
            return Err(PlayerError::WarmupPlaying);
        }
        Ok(())
    }

    // A warmup beatmap osu! has described already.
    pub fn warmup_known(&self, beatmap: i64) -> Option<WarmupMap> {
        self.warmups.get(&beatmap).copied()
    }

    // The team's warmup: the bot puts it on with Freemod, and it starts once everyone is
    // ready, or when the team's turn runs out. Chosen again, the new one replaces it.
    fn choose_warmup(
        &mut self,
        s: &Snapshot,
        p: &RosterPlayer,
        beatmap: i64,
        map: WarmupMap,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        self.warmups.insert(beatmap, map);
        self.check_warmup(s, p.user_id)?;
        if s.in_pool(beatmap, Some(map.beatmapset_id)) {
            return Err(PlayerError::WarmupInPool);
        }
        if map.seconds > WARMUP_MAX_SECS {
            return Err(PlayerError::WarmupTooLong);
        }
        let Phase::Warmup {
            team,
            until,
            skipped,
            ..
        } = self.phase.clone()
        else {
            return Err(PlayerError::NotYourWarmup);
        };
        let name = s.teams[team].name.clone();
        send(out, map_command(beatmap, "osu"));
        send(out, mods_command("", true, false));
        log(
            out,
            LogLevel::Info,
            format!("{name} warmup seçti: {beatmap} ({}).", p.username),
        );
        self.phase = Phase::Warmup {
            team,
            until,
            played: false,
            skipped,
            beatmap: Some(beatmap),
            starting: false,
        };
        let left = (until - s.now).whole_seconds();
        if left < WARMUP_LEAD {
            self.start_countdown(
                s,
                CountdownKind::Warmup,
                WARMUP_LEAD,
                format!(
                    "{name} warmup mapini seçti. Herkes hazır olduğunda ya da {} sonra başlayacak.",
                    span(WARMUP_LEAD)
                ),
                out,
            );
        } else {
            send(
                out,
                format!(
                    "{name} warmup mapini seçti. Herkes hazır olduğunda ya da süre dolunca \
                     başlayacak."
                ),
            );
        }
        Ok(())
    }

    fn skip_warmup(
        &mut self,
        s: &Snapshot,
        p: &RosterPlayer,
        out: &mut Vec<Action>,
    ) -> Result<(), PlayerError> {
        let Phase::Warmup {
            team,
            until,
            played,
            mut skipped,
            beatmap,
            starting,
        } = self.phase.clone()
        else {
            return Err(PlayerError::NotYourWarmup);
        };
        let Some(i) = s.teams.iter().position(|t| t.id == p.team_id) else {
            return Err(PlayerError::NotInMatch);
        };
        let name = &s.teams[i].name;
        if i == team {
            if played || s.room.playing {
                return Err(PlayerError::WarmupPlaying);
            }
            send(out, format!("{name} warmupı geçti."));
            log(
                out,
                LogLevel::Info,
                format!("{name} warmupı geçti ({}).", p.username),
            );
            self.next_warmup(s, team, out);
        } else if i > team {
            if !skipped[i] {
                skipped[i] = true;
                send(out, format!("{name} sırası gelince warmupı geçecek."));
                self.phase = Phase::Warmup {
                    team,
                    until,
                    played,
                    skipped,
                    beatmap,
                    starting,
                };
            }
        } else {
            return Err(PlayerError::NotYourWarmup);
        }
        Ok(())
    }
}

// What the room hears when a player's !abort is refused.
fn abort_refusal(err: PlayerError, name: &str) -> String {
    match err {
        PlayerError::AbortTooLate => "Maç başladıktan 30 saniye sonra abort kullanılamaz.".into(),
        PlayerError::NoAbortsLeft => format!("{name}: takımınızın abort hakkı kalmadı."),
        _ => format!("{name}: şu an oynanan bir map yok."),
    }
}
