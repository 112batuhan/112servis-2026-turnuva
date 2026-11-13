//! Where the tournament is: each stage marked done, current or upcoming, for the landing
//! page's track and the stage tabs of the public pool and statistics pages
//! (`GET /api/public/stages`, handlers/stages.rs). The rule lives only here.
//!
//! The current stage is the qualifier while a lobby is open and no bracket match is over
//! (once one is played the qualifiers are over, even if a lobby was never closed), else
//! the stage of the earliest unfinished bracket match. Before it, every stage is done;
//! after it, a stage is done once it's finished: the qualifier when it had lobbies and
//! none is open or the bracket has started, a bracket stage when it has matches and all
//! of them are concluded.

use serde::Serialize;
use uuid::Uuid;

use crate::db::stages::{PublicStageRow, TournamentProgress};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StageState {
    Done,
    Current,
    Upcoming,
}

// A stage as the public pages see it: unpublished ones too (the track names every round),
// whose pools those pages don't offer.
#[derive(Debug, Serialize)]
pub struct PublicStage {
    pub id: Uuid,
    pub name: String,
    pub position: i32,
    pub published: bool,
    pub is_qualifier: bool,
    pub best_of: Option<i32>,
    // Its matches with a saved map (the qualifier's: its lobbies).
    pub played: i64,
    // Its matches being played now: with a room, not concluded.
    pub live: i64,
    pub state: StageState,
}

// `rows` in play order, the qualifier first (db::stages::list_public_stages).
pub fn track(rows: Vec<PublicStageRow>, progress: &TournamentProgress) -> Vec<PublicStage> {
    let bracket_started = progress.bracket_started;
    let qualifiers_open = progress.open_lobbies > 0;
    let current_id = if qualifiers_open && !bracket_started {
        rows.iter().find(|r| r.is_qualifier).map(|r| r.id)
    } else {
        progress.next_stage_id
    };
    let current_index = rows.iter().position(|r| Some(r.id) == current_id);

    rows.into_iter()
        .enumerate()
        .map(|(i, r)| {
            let finished = if r.is_qualifier {
                (progress.lobbies > 0 && !qualifiers_open) || bracket_started
            } else {
                r.matches > 0 && r.unfinished == 0
            };
            let state = if Some(r.id) == current_id {
                StageState::Current
            } else if finished || current_index.is_some_and(|c| i < c) {
                StageState::Done
            } else {
                StageState::Upcoming
            };
            PublicStage {
                id: r.id,
                name: r.name,
                position: r.position,
                published: r.published,
                is_qualifier: r.is_qualifier,
                best_of: r.best_of,
                played: r.played,
                live: r.live,
                state,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage(n: u128, qualifier: bool, matches: i64, unfinished: i64) -> PublicStageRow {
        PublicStageRow {
            id: Uuid::from_u128(n),
            name: format!("S{n}"),
            position: n as i32,
            published: true,
            is_qualifier: qualifier,
            best_of: None,
            played: 0,
            live: 0,
            matches,
            unfinished,
        }
    }

    // The qualifier, then three bracket stages.
    fn rows(r1: (i64, i64), r2: (i64, i64), r3: (i64, i64)) -> Vec<PublicStageRow> {
        vec![
            stage(0, true, 0, 0),
            stage(1, false, r1.0, r1.1),
            stage(2, false, r2.0, r2.1),
            stage(3, false, r3.0, r3.1),
        ]
    }

    fn states(rows: Vec<PublicStageRow>, progress: TournamentProgress) -> Vec<StageState> {
        track(rows, &progress)
            .into_iter()
            .map(|s| s.state)
            .collect()
    }

    use StageState::{Current as C, Done as D, Upcoming as U};

    #[test]
    fn nothing_scheduled_is_all_upcoming() {
        let got = states(rows((0, 0), (0, 0), (0, 0)), TournamentProgress::default());
        assert_eq!(got, [U, U, U, U]);
    }

    #[test]
    fn open_lobbies_make_the_qualifier_current() {
        let progress = TournamentProgress {
            lobbies: 4,
            open_lobbies: 2,
            bracket_started: false,
            next_stage_id: Some(Uuid::from_u128(1)),
        };
        assert_eq!(states(rows((8, 8), (4, 4), (2, 2)), progress), [C, U, U, U]);
    }

    #[test]
    fn a_played_bracket_match_ends_the_qualifier_even_with_a_lobby_open() {
        let progress = TournamentProgress {
            lobbies: 4,
            open_lobbies: 1,
            bracket_started: true,
            next_stage_id: Some(Uuid::from_u128(2)),
        };
        assert_eq!(states(rows((8, 0), (4, 3), (2, 2)), progress), [D, D, C, U]);
    }

    #[test]
    fn closed_lobbies_finish_the_qualifier_before_the_bracket_starts() {
        let progress = TournamentProgress {
            lobbies: 4,
            open_lobbies: 0,
            bracket_started: false,
            next_stage_id: Some(Uuid::from_u128(1)),
        };
        assert_eq!(states(rows((8, 8), (4, 4), (2, 2)), progress), [D, C, U, U]);
    }

    #[test]
    fn a_stage_before_the_current_one_is_done_even_if_unfinished() {
        // An unscheduled match left in round 1 sorts after round 2's scheduled ones.
        let progress = TournamentProgress {
            lobbies: 4,
            open_lobbies: 0,
            bracket_started: true,
            next_stage_id: Some(Uuid::from_u128(2)),
        };
        assert_eq!(states(rows((8, 1), (4, 4), (2, 2)), progress), [D, D, C, U]);
    }

    #[test]
    fn a_next_match_without_a_stage_leaves_no_stage_current() {
        let progress = TournamentProgress {
            lobbies: 4,
            open_lobbies: 0,
            bracket_started: true,
            next_stage_id: None,
        };
        assert_eq!(states(rows((8, 0), (4, 0), (0, 0)), progress), [D, D, D, U]);
    }

    #[test]
    fn a_qualifier_without_lobbies_goes_by_its_place_on_the_track() {
        let mut progress = TournamentProgress {
            next_stage_id: Some(Uuid::from_u128(1)),
            ..TournamentProgress::default()
        };
        assert_eq!(
            states(rows((8, 8), (0, 0), (0, 0)), progress.clone()),
            [D, C, U, U]
        );
        progress.next_stage_id = None;
        assert_eq!(states(rows((0, 0), (0, 0), (0, 0)), progress), [U, U, U, U]);
    }
}
