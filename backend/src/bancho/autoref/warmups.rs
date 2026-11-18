//! The warmups and the roll.

use crate::bancho::room::commands::player_ref;
use crate::db::autoref::LogLevel;

use super::{
    log, send, span, Action, AutoRef, CountdownKind, Phase, Snapshot, ROLL_TIME, WARMUP_TIME,
};

impl AutoRef {
    pub(super) fn enter_warmup(
        &mut self,
        s: &Snapshot,
        team: usize,
        skipped: [bool; 2],
        out: &mut Vec<Action>,
    ) {
        if team >= 2 || team >= s.teams.len() {
            send(out, "!mp clearhost");
            return self.enter_roll(s, out);
        }
        let t = &s.teams[team];
        if skipped[team] {
            log(out, LogLevel::Info, format!("{} warmupı geçti.", t.name));
            return self.enter_warmup(s, team + 1, skipped, out);
        }
        // The team's first player, or whichever of them is in the room.
        let host = s
            .members(t.id)
            .find(|p| s.present(p))
            .or_else(|| s.members(t.id).next());
        let Some(host) = host else {
            return self.enter_warmup(s, team + 1, skipped, out);
        };
        send(out, format!("!mp host {}", player_ref(host.osu_id)));
        log(
            out,
            LogLevel::Info,
            format!("Warmup: host {} ({}).", host.username, t.name),
        );
        self.phase = Phase::Warmup {
            team,
            until: s.now + WARMUP_TIME,
            played: false,
            skipped,
        };
        let secs = WARMUP_TIME.whole_seconds();
        self.start_countdown(
            s,
            CountdownKind::Warmup,
            secs,
            format!(
                "{} takımına warmup açmaları için host yetkisi verildi, {} süreleri var. Warmup \
                 mapleri 4 dakikayı geçmemelidir. Eğer warmup oynamak istemiyorlarsa -warmup-gec \
                 yazabilirler.",
                t.name,
                span(secs)
            ),
            out,
        );
    }

    pub(super) fn next_warmup(&mut self, s: &Snapshot, team: usize, out: &mut Vec<Action>) {
        let skipped = match &self.phase {
            Phase::Warmup { skipped, .. } => *skipped,
            _ => [false, false],
        };
        self.enter_warmup(s, team + 1, skipped, out);
    }

    fn enter_roll(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        log(out, LogLevel::Info, "Roll bekleniyor.");
        self.start_roll(
            s,
            "Warmuplar bitti, pick ve ban sırasını belirlemek için !roll komutunu kullanın,",
            out,
        );
    }

    // The roll's two minutes start over, the room told by `lead` and how long it has.
    pub(super) fn start_roll(&mut self, s: &Snapshot, lead: &str, out: &mut Vec<Action>) {
        self.phase = Phase::Roll {
            until: s.now + ROLL_TIME,
            rolls: [None, None],
        };
        let secs = ROLL_TIME.whole_seconds();
        self.start_countdown(
            s,
            CountdownKind::Roll,
            secs,
            format!("{lead} {} süreniz var.", span(secs)),
            out,
        );
    }

    pub(super) fn take_roll(&mut self, s: &Snapshot, name: &str, n: i32, out: &mut Vec<Action>) {
        let Phase::Roll { until, mut rolls } = self.phase.clone() else {
            return;
        };
        let Some(p) = s.player(name) else {
            return;
        };
        let Some(i) = s.teams.iter().position(|t| t.id == p.team_id) else {
            return;
        };
        if rolls[i].is_some() {
            return;
        }
        if n > 100 {
            send(
                out,
                format!(
                    "{}: sadece !roll yazın (1-100), bu roll sayılmadı.",
                    p.username
                ),
            );
            return;
        }
        rolls[i] = Some(n);
        log(
            out,
            LogLevel::Info,
            format!("{} ({}) {n} attı.", p.username, s.teams[i].name),
        );
        self.phase = Phase::Roll { until, rolls };
        if rolls.iter().all(Option::is_some) {
            self.resolve_roll(s, out);
        }
    }

    pub(super) fn resolve_roll(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        let Phase::Roll { rolls, .. } = self.phase.clone() else {
            return;
        };
        if s.teams.len() < 2 {
            return;
        }
        let (winner, line) = match rolls {
            [Some(a), Some(b)] if a == b => {
                let lead = format!("Berabere ({a} - {b})! İki takım da tekrar !roll yazsın,");
                return self.start_roll(s, &lead, out);
            }
            [Some(a), Some(b)] => (if a > b { 0 } else { 1 }, format!("({a} - {b})")),
            [Some(_), None] => (0, "(rakibi roll atmadı)".into()),
            [None, Some(_)] => (1, "(rakibi roll atmadı)".into()),
            [None, None] => (
                rand::random::<bool>() as usize,
                "(iki takım da roll atmadı, bot rastgele seçti)".into(),
            ),
        };
        let won = &s.teams[winner];
        let lost = &s.teams[1 - winner];
        send(out, format!("{} rollü kazandı {line}.", won.name));
        log(
            out,
            LogLevel::Info,
            format!("Roll: {} kazandı {line}.", won.name),
        );
        let first = if s.first_step_team == 1 { won } else { lost };
        out.push(Action::FirstTurn(first.id));
        self.phase = Phase::Next;
    }
}
