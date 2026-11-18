//! Between maps: whose turn it is to ban or pick, and the end of the match.

use uuid::Uuid;

use crate::bancho::{
    context::MatchTeam,
    tally::{score_message, standing_of, PoolEntry, Standing},
};
use crate::{
    category,
    db::autoref::LogLevel,
    draft::{StepKind, Turn},
};

use super::{
    kind_word, log, send, span, Action, AutoRef, CountdownKind, Phase, Snapshot, BAN_TIME, CLOSING,
    PICK_TIME,
};

impl AutoRef {
    pub(super) fn next_step(&mut self, s: &Snapshot, out: &mut Vec<Action>) {
        if !s.bracket {
            let next = s
                .entries
                .iter()
                .find(|e| !e.tiebreaker && !s.played.contains(&e.map_id));
            match next {
                Some(entry) => self.enter_ready(s, entry, out),
                None => self.finish(s, None, out),
            }
            return;
        }
        if s.teams.len() < 2 {
            out.push(Action::Handover("Maçın iki takımı belli değil.".into()));
            return;
        }
        if let (Some(best_of), Some((a, b))) = (s.best_of, s.score) {
            match standing_of(best_of, a, b) {
                Standing::Won => {
                    let winner = if a > b { &s.teams[0] } else { &s.teams[1] };
                    return self.finish(s, Some(winner), out);
                }
                Standing::Over => {
                    out.push(Action::Handover(format!(
                        "Skor ({a} - {b}) Bo{best_of}'u aşıyor."
                    )));
                    return;
                }
                Standing::Tiebreaker => {
                    let Some(tb) = s.entries.iter().find(|e| e.tiebreaker) else {
                        out.push(Action::Handover(
                            "Skor tiebreaker'a geldi ama havuzda tiebreaker yok.".into(),
                        ));
                        return;
                    };
                    if s.played.contains(&tb.map_id) {
                        out.push(Action::Handover(
                            "Tiebreaker oynandı ama kazanan belli değil.".into(),
                        ));
                        return;
                    }
                    send(out, "Tiebreaker!");
                    log(out, LogLevel::Info, "Skor eşit, tiebreaker açılıyor.");
                    return self.enter_ready(s, tb, out);
                }
                Standing::Open | Standing::MatchPoint => {}
            }
        }
        // A pick that hasn't been played yet (one a referee marked by hand in manual
        // mode) is played before the next turn.
        if let Some(entry) = s
            .picks
            .iter()
            .filter(|p| !s.finished.contains(&p.map_id))
            .min_by_key(|p| p.created_at)
            .and_then(|p| s.entry(p.map_id))
        {
            log(
                out,
                LogLevel::Info,
                format!("{} seçildi ama henüz oynanmadı, açılıyor.", entry.label),
            );
            return self.enter_ready(s, entry, out);
        }
        match s.turn {
            Some(Turn {
                kind,
                team_id: Some(team_id),
            }) => self.ask(s, team_id, kind, out),
            Some(_) => out.push(Action::Handover(
                "Sıradaki ban/pick'in takımı belirlenemedi.".into(),
            )),
            None if s.best_of.is_none() => out.push(Action::Handover(
                "Stage'in best of değeri yok, maçın ne zaman bittiği bilinmiyor.".into(),
            )),
            None => out.push(Action::Handover(
                "Stage'in pick sırası bitti ama maç bitmedi.".into(),
            )),
        }
    }

    // Asks the team for its ban or pick, on the clock: a ban that runs out is lost, a
    // pick passes to the other team (or, already passed, goes to a referee).
    fn ask(&mut self, s: &Snapshot, team_id: Uuid, kind: StepKind, out: &mut Vec<Action>) {
        let name = s.team_name(team_id);
        log(
            out,
            LogLevel::Info,
            format!("{name} takımının {} seçimi bekleniyor.", kind_word(kind)),
        );
        let time = match kind {
            StepKind::Ban => BAN_TIME,
            StepKind::Pick => PICK_TIME,
        };
        let secs = time.whole_seconds();
        let passed = s.passed_from().is_some_and(|t| t != team_id);
        let then = match kind {
            StepKind::Ban => ", süre dolarsa ban hakkınız yanar",
            StepKind::Pick if passed => "",
            StepKind::Pick => ", süre dolarsa pick sırası rakibe geçer",
        };
        self.phase = Phase::Draft {
            team_id,
            kind,
            candidate: None,
            until: s.now + time,
        };
        self.start_countdown(
            s,
            CountdownKind::Draft,
            secs,
            format!(
                "{name} takımı {} sırası, seçtiğiniz mapin slotunu söyleyin. {} süreniz var{then}.",
                kind_word(kind),
                span(secs)
            ),
            out,
        );
    }

    // The team let its ban or pick run out.
    pub(super) fn missed(
        &mut self,
        s: &Snapshot,
        team_id: Uuid,
        kind: StepKind,
        out: &mut Vec<Action>,
    ) {
        let name = s.team_name(team_id);
        match kind {
            StepKind::Ban => {
                send(
                    out,
                    format!("{name} takımının ban süresi doldu, ban hakkı yandı."),
                );
                log(
                    out,
                    LogLevel::Info,
                    format!("{name} ban süresini kaçırdı, ban hakkı yandı."),
                );
            }
            StepKind::Pick if s.passed_from().is_some_and(|t| t != team_id) => {
                send(out, format!("{name} takımının pick süresi doldu."));
                out.push(Action::Handover(
                    "Pick süresi iki takımda da doldu: ilk takımın pick'i rakibe geçti, o da seçmedi."
                        .into(),
                ));
                return;
            }
            StepKind::Pick => {
                let other = s
                    .teams
                    .iter()
                    .find(|t| t.id != team_id)
                    .map_or("?", |t| t.name.as_str());
                send(
                    out,
                    format!(
                        "{name} takımının pick süresi doldu, pick sırası {other} takımına geçti."
                    ),
                );
                log(
                    out,
                    LogLevel::Info,
                    format!("{name} pick süresini kaçırdı, pick {other} takımına geçti."),
                );
            }
        }
        out.push(Action::Missed { team_id, kind });
        self.phase = Phase::Next;
    }

    fn finish(&mut self, s: &Snapshot, winner: Option<&MatchTeam>, out: &mut Vec<Action>) {
        match (winner, s.score) {
            (Some(w), Some((a, b))) => {
                let names = [s.teams[0].name.as_str(), s.teams[1].name.as_str()];
                send(
                    out,
                    format!(
                        "{} maçı kazandı! {}",
                        w.name,
                        score_message(names, [a, b], None)
                    ),
                );
                log(
                    out,
                    LogLevel::Info,
                    format!("{} kazandı ({a} - {b}).", w.name),
                );
            }
            _ => {
                send(out, "Bütün mapler oynandı, katıldığınız için teşekkürler!");
                log(out, LogLevel::Info, "Bütün mapler oynandı.");
            }
        }
        send(out, CLOSING);
        out.push(Action::Conclude);
        self.phase = Phase::Done;
    }
}

// Why a team can't ban or pick this map, in the words the room hears.
pub(super) fn refusal(
    s: &Snapshot,
    entry: &PoolEntry,
    kind: StepKind,
    team_id: Uuid,
) -> Option<String> {
    let label = &entry.label;
    if entry.tiebreaker {
        let verb = match kind {
            StepKind::Pick => "picklenemez",
            StepKind::Ban => "banlanamaz",
        };
        return Some(format!("{label} tiebreaker, {verb}."));
    }
    if s.bans.iter().any(|b| b.map_id == entry.map_id) {
        return Some(format!("{label} banlandı, başka bir map seçin."));
    }
    if s.picks.iter().any(|p| p.map_id == entry.map_id) || s.played.contains(&entry.map_id) {
        return Some(format!("{label} zaten oynandı, başka bir map seçin."));
    }
    if kind == StepKind::Pick && !category::repeatable(&entry.category) {
        let last = s
            .picks
            .iter()
            .filter(|p| p.team_id == team_id)
            .max_by_key(|p| p.created_at)
            .and_then(|p| s.entry(p.map_id));
        if last.is_some_and(|l| l.category == entry.category) {
            return Some(format!(
                "Aynı moddan ({}) arka arkaya pick yapamazsınız (NM hariç), başka bir mod seçin.",
                entry.category
            ));
        }
    }
    None
}
