//! Where a match stands: who's ahead, whose turn it is, and the line the room hears.

use uuid::Uuid;

use crate::{
    db::bracket::{BanRow, MissedRow, PickRow},
    draft::{next_turn, DraftOrder, StepKind, Taken, Turn},
};

// Where a bracket match stands against its best of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Open,
    MatchPoint,
    // Both teams are a map away, so the tiebreaker is played.
    Tiebreaker,
    Won,
    Over,
}

pub fn standing_of(best_of: i32, team1: i32, team2: i32) -> Standing {
    let need = (best_of + 1) / 2;
    if team1 > need || team2 > need || (team1 >= need && team2 >= need) {
        return Standing::Over;
    }
    if team1 == need || team2 == need {
        return Standing::Won;
    }
    if need > 1 && team1 == need - 1 && team2 == need - 1 {
        return Standing::Tiebreaker;
    }
    if team1 == need - 1 || team2 == need - 1 {
        Standing::MatchPoint
    } else {
        Standing::Open
    }
}

// Whose turn it is to ban or pick, and which: the stage's order read against the bans
// and picks so far and the turns that ran out, in the order they came (draft::next_turn).
pub fn turn_of(
    order: &DraftOrder,
    bans: &[BanRow],
    picks: &[PickRow],
    missed: &[MissedRow],
    teams: [Uuid; 2],
    chosen: Option<Uuid>,
) -> Option<Turn> {
    let mut taken: Vec<_> = bans
        .iter()
        .map(|b| (b.created_at, Taken::made(b.team_id)))
        .chain(picks.iter().map(|p| (p.created_at, Taken::made(p.team_id))))
        .chain(missed.iter().map(|m| {
            let taken = Taken {
                team_id: m.team_id,
                passed: m.kind == StepKind::Pick,
            };
            (m.created_at, taken)
        }))
        .collect();
    taken.sort_by_key(|t| t.0);
    let taken: Vec<Taken> = taken.into_iter().map(|(_, t)| t).collect();
    next_turn(order, &taken, teams, chosen)
}

// The line sent to the room: "Skor: Team A 2 - 1 Team B | Sıradaki pick: Team B" (or
// "Sıradaki ban").
pub fn score_message(names: [&str; 2], score: [i32; 2], next: Option<(StepKind, &str)>) -> String {
    let line = format!(
        "Skor: {} {} - {} {}",
        names[0], score[0], score[1], names[1]
    );
    match next {
        Some((StepKind::Pick, team)) => format!("{line} | Sıradaki pick: {team}"),
        Some((StepKind::Ban, team)) => format!("{line} | Sıradaki ban: {team}"),
        None => line,
    }
}
