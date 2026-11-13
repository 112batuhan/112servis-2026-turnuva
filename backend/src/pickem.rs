//! Pick'em: when each stage takes predictions, and what a prediction scores.
//!
//! A stage's pick'em opens once every match of the stage before it (the qualifier lobbies,
//! for the first bracket stage) is concluded and locks as its first match starts, unless a
//! host has opened or locked it by hand. A stage a host has opened stays open while any of
//! its matches is still to be played, each match locking on its own once its room opens or
//! it has a result (the clock doesn't lock it, so a match running late or one whose time
//! was set in the past can still be predicted), so a stage already under way can be opened
//! again for the rest of its matches. A prediction is a winner and a score, and scores
//! 3 with both right, 2 with only the winner right, 1 with only the score right (the
//! figures, whoever won), else 0. A forfeit has no score, so only its winner can score.

use std::collections::HashMap;

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::db::pickem::{MatchRow, Mode, PredictionRow, StageRow};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    // Waiting for the stage before it to be played.
    Upcoming,
    // Taking predictions.
    Open,
    // No more predictions; the split is public and results come in.
    Locked,
    // Locked with every match concluded.
    Finished,
}

impl Status {
    // Whether the stage's predictions are past changing, so their split can be shown.
    pub fn locked(self) -> bool {
        matches!(self, Status::Locked | Status::Finished)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub stage_id: Uuid,
    // How it opens and locks, as a host set it.
    pub mode: Mode,
    pub status: Status,
    // When the stage before it was played out; None while it isn't, or without one.
    pub opens_at: Option<OffsetDateTime>,
    // When predictions next lock: its first match's time, or for a stage a host has opened,
    // the next time still to come of a match still open (each locks on its own). None while
    // none is scheduled.
    pub closes_at: Option<OffsetDateTime>,
    // The stage it waits for.
    pub previous: Option<Uuid>,
}

// Whether a match has begun in a way that locks its stage: its time has come, a room is
// open for it or it's over.
pub fn started(m: &MatchRow, now: OffsetDateTime) -> bool {
    m.concluded_at.is_some() || m.has_room || m.starts_at.is_some_and(|t| t <= now)
}

// Whether a match takes no more predictions. By the rule it's once it has started; in a
// stage a host opened the clock is overridden, so only its room opening or its result
// locks it.
pub fn match_locked(m: &MatchRow, mode: Mode, now: OffsetDateTime) -> bool {
    match mode {
        Mode::Open => m.concluded_at.is_some() || m.has_room,
        Mode::Auto | Mode::Closed => started(m, now),
    }
}

// The pick'em window of every stage that has bracket matches and isn't the qualifier
// stage, in stage order. `lobbies` are the qualifier lobbies' conclusion times.
pub fn windows(
    stages: &[StageRow],
    matches: &[MatchRow],
    lobbies: &[Option<OffsetDateTime>],
    now: OffsetDateTime,
) -> Vec<Window> {
    let mut by_stage: HashMap<Uuid, Vec<&MatchRow>> = HashMap::new();
    for m in matches {
        if let Some(stage_id) = m.stage_id {
            by_stage.entry(stage_id).or_default().push(m);
        }
    }
    // Each stage's matches' conclusion times, for the stage after it to wait on.
    let played = |stage: &StageRow| -> Vec<Option<OffsetDateTime>> {
        if stage.is_qualifier {
            lobbies.to_vec()
        } else {
            by_stage
                .get(&stage.id)
                .map(|ms| ms.iter().map(|m| m.concluded_at).collect())
                .unwrap_or_default()
        }
    };

    let mut out = Vec::new();
    for (i, stage) in stages.iter().enumerate() {
        let Some(own) = by_stage.get(&stage.id) else {
            continue;
        };
        if stage.is_qualifier {
            continue;
        }
        // The nearest stage before it that has any matches.
        let previous = stages[..i].iter().rev().find(|s| !played(s).is_empty());
        let (prev_done, opens_at) = match previous {
            Some(prev) => {
                let times = played(prev);
                let done = times.iter().all(Option::is_some);
                (
                    done,
                    if done {
                        times.into_iter().flatten().max()
                    } else {
                        None
                    },
                )
            }
            None => (true, None),
        };
        let mode = stage.pickem_mode;
        // Opened by hand, it only locks once every match is locked on its own.
        let locked = match mode {
            Mode::Closed => true,
            Mode::Open => own.iter().all(|m| match_locked(m, mode, now)),
            Mode::Auto => own.iter().any(|m| started(m, now)),
        };
        let closes_at = if mode == Mode::Open && !locked {
            own.iter()
                .filter(|m| !match_locked(m, mode, now))
                .filter_map(|m| m.starts_at)
                .filter(|&t| t > now)
                .min()
        } else {
            own.iter().filter_map(|m| m.starts_at).min()
        };
        let status = if locked {
            if own.iter().all(|m| m.concluded_at.is_some()) {
                Status::Finished
            } else {
                Status::Locked
            }
        } else if mode == Mode::Open || prev_done {
            Status::Open
        } else {
            Status::Upcoming
        };
        out.push(Window {
            stage_id: stage.id,
            mode,
            status,
            opens_at,
            closes_at,
            previous: previous.map(|s| s.id),
        });
    }
    out
}

// Whether how a match was predicted can be shown: its stage is locked, or the match is
// locked on its own in a stage a host opened, so its predictions can't change any more.
pub fn revealed(w: &Window, m: &MatchRow, now: OffsetDateTime) -> bool {
    w.status.locked() || (w.status == Status::Open && match_locked(m, w.mode, now))
}

// Whether a match can be predicted now: its stage is open, both teams are known, the match
// isn't locked, and there's a best of to say what the scores can be.
pub fn predictable(w: &Window, m: &MatchRow, best_of: Option<i32>, now: OffsetDateTime) -> bool {
    w.status == Status::Open
        && m.team1_id.is_some()
        && m.team2_id.is_some()
        && best_of.is_some()
        && !match_locked(m, w.mode, now)
}

// Map wins a team needs under a best of.
pub fn first_to(best_of: i32) -> i32 {
    (best_of + 1) / 2
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub winner: Uuid,
    // The winner's maps, then the loser's; None for a forfeit.
    pub score: Option<(i32, i32)>,
}

// How a concluded match ended, as its conclusion decided it.
pub fn outcome(m: &MatchRow) -> Option<Outcome> {
    m.concluded_at?;
    let (t1, t2) = (m.team1_id?, m.team2_id?);
    match m.forfeit_slot {
        Some(1) => {
            return Some(Outcome {
                winner: t2,
                score: None,
            })
        }
        Some(_) => {
            return Some(Outcome {
                winner: t1,
                score: None,
            })
        }
        None => {}
    }
    let (s1, s2) = (m.team1_score?, m.team2_score?);
    match s1.cmp(&s2) {
        std::cmp::Ordering::Greater => Some(Outcome {
            winner: t1,
            score: Some((s1, s2)),
        }),
        std::cmp::Ordering::Less => Some(Outcome {
            winner: t2,
            score: Some((s2, s1)),
        }),
        std::cmp::Ordering::Equal => None,
    }
}

pub const POINTS_EXACT: i32 = 3;
pub const POINTS_WINNER: i32 = 2;
pub const POINTS_SCORE: i32 = 1;

// What a prediction scored.
pub fn points(p: &PredictionRow, o: &Outcome) -> i32 {
    let winner = p.team_id == o.winner;
    let score = o.score == Some((p.winner_score, p.loser_score));
    match (winner, score) {
        (true, true) => POINTS_EXACT,
        (true, false) => POINTS_WINNER,
        (false, true) => POINTS_SCORE,
        (false, false) => 0,
    }
}

// Whether a prediction is still on one of the match's teams.
pub fn counts(p: &PredictionRow, m: &MatchRow) -> bool {
    Some(p.team_id) == m.team1_id || Some(p.team_id) == m.team2_id
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Tally {
    pub points: i32,
    // Predictions that got both right.
    pub exact: i32,
    // Predictions that got the winner right.
    pub winners: i32,
    // Predictions on matches that have a result.
    pub scored: i32,
    // Every prediction made.
    pub picks: i32,
}

impl Tally {
    fn add(&mut self, scored: Option<i32>) {
        self.picks += 1;
        if let Some(points) = scored {
            self.scored += 1;
            self.points += points;
            if points == POINTS_EXACT {
                self.exact += 1;
            }
            if points >= POINTS_WINNER {
                self.winners += 1;
            }
        }
    }
}

// Each user's tally overall and per stage. A prediction counts once its match has a
// result; one left on a team that's no longer in the match scores nothing.
pub fn tallies(
    predictions: &[PredictionRow],
    matches: &HashMap<Uuid, &MatchRow>,
) -> HashMap<Uuid, (Tally, HashMap<Uuid, Tally>)> {
    let mut out: HashMap<Uuid, (Tally, HashMap<Uuid, Tally>)> = HashMap::new();
    for p in predictions {
        let Some(m) = matches.get(&p.match_id) else {
            continue;
        };
        let Some(stage_id) = m.stage_id else {
            continue;
        };
        let scored = outcome(m).map(|o| if counts(p, m) { points(p, &o) } else { 0 });
        let entry = out.entry(p.user_id).or_default();
        entry.0.add(scored);
        entry.1.entry(stage_id).or_default().add(scored);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use time::Duration;

    static NEXT: AtomicU64 = AtomicU64::new(1000);

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn stage(n: u128, position: i32, mode: Mode) -> StageRow {
        StageRow {
            id: id(n),
            name: format!("Stage {n}"),
            position,
            is_qualifier: false,
            best_of: Some(9),
            pickem_mode: mode,
        }
    }

    fn game(stage: u128, starts_in: Option<i64>, concluded: bool, now: OffsetDateTime) -> MatchRow {
        MatchRow {
            id: id(NEXT.fetch_add(1, Ordering::Relaxed).into()),
            stage_id: Some(id(stage)),
            starts_at: starts_in.map(|h| now + Duration::hours(h)),
            team1_id: Some(id(100)),
            team2_id: Some(id(200)),
            team1_score: concluded.then_some(5),
            team2_score: concluded.then_some(3),
            forfeit_slot: None,
            concluded_at: concluded.then_some(now - Duration::hours(1)),
            has_room: concluded,
        }
    }

    fn pick(team: u128, winner_score: i32, loser_score: i32) -> PredictionRow {
        PredictionRow {
            user_id: id(1),
            match_id: id(2),
            team_id: id(team),
            winner_score,
            loser_score,
        }
    }

    #[test]
    fn points_follow_the_rules() {
        let o = Outcome {
            winner: id(100),
            score: Some((5, 3)),
        };
        assert_eq!(points(&pick(100, 5, 3), &o), 3);
        assert_eq!(points(&pick(100, 5, 4), &o), 2);
        assert_eq!(points(&pick(200, 5, 3), &o), 1);
        assert_eq!(points(&pick(200, 5, 1), &o), 0);
    }

    #[test]
    fn a_forfeit_only_scores_the_winner() {
        let o = Outcome {
            winner: id(100),
            score: None,
        };
        assert_eq!(points(&pick(100, 5, 0), &o), 2);
        assert_eq!(points(&pick(200, 5, 0), &o), 0);
    }

    #[test]
    fn outcome_orders_the_score_by_winner() {
        let now = OffsetDateTime::now_utc();
        let mut m = game(1, Some(-2), true, now);
        m.team1_score = Some(2);
        m.team2_score = Some(5);
        assert_eq!(
            outcome(&m),
            Some(Outcome {
                winner: id(200),
                score: Some((5, 2))
            })
        );
        m.forfeit_slot = Some(2);
        assert_eq!(
            outcome(&m),
            Some(Outcome {
                winner: id(100),
                score: None
            })
        );
    }

    #[test]
    fn a_stage_opens_after_the_one_before_is_played_and_locks_at_its_first_match() {
        let now = OffsetDateTime::now_utc();
        let stages = [
            stage(1, 0, Mode::Auto),
            stage(2, 1, Mode::Auto),
            stage(3, 2, Mode::Auto),
        ];
        let matches = [
            game(1, Some(-30), true, now),
            game(2, Some(-3), true, now),
            game(2, Some(-1), false, now),
            game(3, Some(20), false, now),
        ];
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(
            w.iter().map(|w| w.status).collect::<Vec<_>>(),
            [Status::Finished, Status::Locked, Status::Upcoming,]
        );

        // Once stage 2 is played out, stage 3 opens.
        let mut matches = matches;
        matches[2] = game(2, Some(-1), true, now);
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[2].status, Status::Open);
        assert_eq!(w[2].previous, Some(id(2)));
        assert_eq!(w[2].closes_at, matches[3].starts_at);
    }

    #[test]
    fn hosts_override_the_rule() {
        let now = OffsetDateTime::now_utc();
        let stages = [
            stage(1, 0, Mode::Auto),
            stage(2, 1, Mode::Open),
            stage(3, 2, Mode::Closed),
        ];
        let matches = [
            game(1, Some(-1), false, now),
            game(2, Some(5), false, now),
            game(3, Some(9), false, now),
        ];
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Open);
        assert_eq!(w[2].status, Status::Locked);
    }

    #[test]
    fn a_stage_opened_by_hand_stays_open_for_the_matches_still_to_start() {
        let now = OffsetDateTime::now_utc();
        let stages = [stage(1, 0, Mode::Auto), stage(2, 1, Mode::Open)];
        let mut matches = vec![
            game(1, Some(-30), true, now),
            game(2, Some(-2), true, now),
            game(2, Some(-1), false, now),
            game(2, Some(3), false, now),
            game(2, Some(6), false, now),
        ];
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Open);
        assert_eq!(w[1].closes_at, matches[3].starts_at);
        assert!(revealed(&w[1], &matches[1], now));
        assert!(!predictable(&w[1], &matches[1], Some(9), now));
        assert!(!revealed(&w[1], &matches[3], now));
        assert!(predictable(&w[1], &matches[3], Some(9), now));
        // The clock doesn't lock it: a match past its time with no room yet is still open.
        assert!(!revealed(&w[1], &matches[2], now));
        assert!(predictable(&w[1], &matches[2], Some(9), now));
        // Its room opening does.
        matches[2].has_room = true;
        let w = windows(&stages, &matches, &[], now);
        assert!(revealed(&w[1], &matches[2], now));
        assert!(!predictable(&w[1], &matches[2], Some(9), now));

        // Once every match has a room it locks, and finishes once they're all played.
        matches[3].has_room = true;
        matches[4].has_room = true;
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Locked);
        for m in &mut matches[2..] {
            m.concluded_at = Some(now);
        }
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Finished);

        // Handed back to the rule, a stage under way locks.
        let stages = [stage(1, 0, Mode::Auto), stage(2, 1, Mode::Auto)];
        let matches = [
            game(1, Some(-30), true, now),
            game(2, Some(-1), false, now),
            game(2, Some(3), false, now),
        ];
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Locked);
    }

    #[test]
    fn opening_by_hand_reopens_a_stage_whose_times_have_passed() {
        let now = OffsetDateTime::now_utc();
        let matches = [
            game(1, Some(-30), true, now),
            game(2, Some(-3), false, now),
            game(2, Some(-2), false, now),
        ];
        // By the rule it's locked: its matches' times have come.
        let stages = [stage(1, 0, Mode::Auto), stage(2, 1, Mode::Auto)];
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Locked);
        // Opened, both matches take predictions until their rooms open, with no time to lock at.
        let stages = [stage(1, 0, Mode::Auto), stage(2, 1, Mode::Open)];
        let w = windows(&stages, &matches, &[], now);
        assert_eq!(w[1].status, Status::Open);
        assert_eq!(w[1].closes_at, None);
        assert!(predictable(&w[1], &matches[1], Some(9), now));
        assert!(predictable(&w[1], &matches[2], Some(9), now));
    }

    #[test]
    fn the_first_bracket_stage_waits_for_the_qualifiers() {
        let now = OffsetDateTime::now_utc();
        let mut quals = stage(1, 0, Mode::Auto);
        quals.is_qualifier = true;
        let stages = [quals, stage(2, 1, Mode::Auto)];
        let matches = [game(2, Some(10), false, now)];
        let w = windows(&stages, &matches, &[Some(now), None], now);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].status, Status::Upcoming);
        let w = windows(&stages, &matches, &[Some(now), Some(now)], now);
        assert_eq!(w[0].status, Status::Open);
        assert_eq!(w[0].opens_at, Some(now));
    }

    #[test]
    fn tallies_count_points_once_matches_have_results() {
        let now = OffsetDateTime::now_utc();
        let done = game(1, Some(-2), true, now);
        let open = game(1, Some(2), false, now);
        let matches: HashMap<Uuid, &MatchRow> = [(done.id, &done), (open.id, &open)].into();
        let predictions = [
            PredictionRow {
                match_id: done.id,
                ..pick(100, 5, 3)
            },
            PredictionRow {
                match_id: open.id,
                ..pick(200, 5, 1)
            },
        ];
        let t = tallies(&predictions, &matches);
        let (total, stages) = &t[&id(1)];
        assert_eq!(
            total,
            &Tally {
                points: 3,
                exact: 1,
                winners: 1,
                scored: 1,
                picks: 2
            }
        );
        assert_eq!(stages[&id(1)].points, 3);
    }
}
