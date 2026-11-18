//! What the players write in the room: their bans and picks and the commands.

use crate::bancho::tally::RosterPlayer;
use crate::{db::autoref::LogLevel, draft::StepKind};

use super::draft::refusal;
use super::{
    command, log, send, Action, AutoRef, Phase, Snapshot, ABORT_WINDOW, REPORT_EVERY, REPORT_MAX,
};

impl AutoRef {
    // A line a player wrote in the room. `enabled`: the match is in auto mode; -rapor
    // and -durdur work either way.
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
        let command = command(text);
        if let Some(("-rapor", rest)) = command.as_ref().map(|(w, r)| (w.as_str(), *r)) {
            return self.report(s, &p, rest, out);
        }
        if command.as_ref().is_some_and(|(w, _)| w == "-durdur") {
            return self.stop(s, &p, out);
        }
        if !enabled || self.phase == Phase::Done {
            return;
        }
        match command.as_ref().map(|(w, _)| w.as_str()) {
            Some("-abort") => self.abort(s, &p, out),
            Some("-warmup-gec") => self.skip_warmup(s, &p, out),
            _ => self.draft_input(s, &p, text, out),
        }
    }

    fn report(&mut self, s: &Snapshot, p: &RosterPlayer, text: &str, out: &mut Vec<Action>) {
        if text.is_empty() {
            send(out, format!("{}: kullanım: -rapor <mesajınız>", p.username));
            return;
        }
        if self
            .reports
            .get(&p.osu_id)
            .is_some_and(|&t| s.now - t < REPORT_EVERY)
        {
            send(
                out,
                format!("{}: bir dakikada bir rapor gönderebilirsiniz.", p.username),
            );
            return;
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
    }

    // -durdur, for emergencies, any number of times: the bot stops (the match goes to
    // manual until a referee switches it back) and the staff hear it. Once a minute per
    // player, so it can't flood them.
    fn stop(&mut self, s: &Snapshot, p: &RosterPlayer, out: &mut Vec<Action>) {
        if self
            .stops
            .get(&p.osu_id)
            .is_some_and(|&t| s.now - t < REPORT_EVERY)
        {
            return;
        }
        self.stops.insert(p.osu_id, s.now);
        // A countdown or a mods check waiting is dropped; it's set again on resuming.
        self.set_ready(|r| {
            r.check_at = None;
            r.starting = None;
        });
        send(
            out,
            "Durdur komutu kullanıldı. Bu komut sadece acil durumlarda kullanılmalıdır. Hostlar \
             bilgilendirildi, eğer bu komut kötü niyetle kullanıldıysa -rapor komutuyla bildiriniz.",
        );
        out.push(Action::Stop {
            user_id: p.user_id,
            team_id: p.team_id,
            username: p.username.clone(),
        });
    }

    fn abort(&mut self, s: &Snapshot, p: &RosterPlayer, out: &mut Vec<Action>) {
        if !matches!(self.phase, Phase::Playing { .. }) || !s.room.playing {
            send(out, format!("{}: şu an oynanan bir map yok.", p.username));
            return;
        }
        let early = s
            .room
            .map_started_at
            .is_some_and(|t| s.now - t <= ABORT_WINDOW);
        if !early {
            send(out, "Maç başladıktan 30 saniye sonra abort kullanılamaz.");
            return;
        }
        if self.aborts_left(p.team_id) == 0 {
            send(
                out,
                format!("{}: takımınızın abort hakkı kalmadı.", p.username),
            );
            return;
        }
        self.uses_mut(p.team_id).aborts += 1;
        let left = self.aborts_left(p.team_id);
        send(out, "!mp abort");
        send(out, format!("Abort kullanıldı. {left} hakkınız kaldı."));
        log(
            out,
            LogLevel::Info,
            format!("{} -abort kullandı ({left} hakkı kaldı).", p.username),
        );
    }

    fn skip_warmup(&mut self, s: &Snapshot, p: &RosterPlayer, out: &mut Vec<Action>) {
        let Phase::Warmup {
            team,
            until,
            played,
            mut skipped,
        } = self.phase.clone()
        else {
            return;
        };
        let Some(i) = s.teams.iter().position(|t| t.id == p.team_id) else {
            return;
        };
        let name = &s.teams[i].name;
        if i == team {
            if s.room.playing {
                return;
            }
            send(out, format!("{name} warmupı geçti."));
            log(out, LogLevel::Info, format!("{name} -warmup-gec yazdı."));
            self.next_warmup(s, team, out);
        } else if i > team && !skipped[i] {
            skipped[i] = true;
            send(out, format!("{name} sırası gelince warmupı geçecek."));
            self.phase = Phase::Warmup {
                team,
                until,
                played,
                skipped,
            };
        }
    }

    // A player of the team whose turn it is naming a slot ("HD1"), then naming it again
    // to confirm.
    fn draft_input(&mut self, s: &Snapshot, p: &RosterPlayer, text: &str, out: &mut Vec<Action>) {
        let Phase::Draft {
            team_id,
            kind,
            candidate,
            until,
        } = self.phase.clone()
        else {
            return;
        };
        if p.team_id != team_id {
            return;
        }
        let Some(entry) = s.entry_named(text) else {
            return;
        };
        if let Some(reason) = refusal(s, entry, kind, team_id) {
            send(out, reason);
            return;
        }
        if candidate != Some(entry.map_id) {
            let word = match kind {
                StepKind::Ban => "Ban",
                StepKind::Pick => "Pick",
            };
            send(
                out,
                format!(
                    "{word} için {} seçtiğinizi doğrulamak için tekrar yazın.",
                    entry.label
                ),
            );
            self.phase = Phase::Draft {
                team_id,
                kind,
                candidate: Some(entry.map_id),
                until,
            };
            return;
        }
        let name = s.team_name(team_id);
        let did = match kind {
            StepKind::Ban => "banladı",
            StepKind::Pick => "seçti",
        };
        log(
            out,
            LogLevel::Info,
            format!("{name} {} {did} ({} onayladı).", entry.label, p.username),
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
                self.enter_ready(s, entry, out);
            }
        }
    }
}
