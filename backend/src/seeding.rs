//! Seeding the bracket from the qualifier: the order the teams are seeded in, and the
//! double-elimination bracket a Round of 32 or a Round of 16 makes of that order, laid
//! out as the dev seed lays its bracket out (backend/seed/seed.sql).
//!
//! The first round pairs the seeds in the standard order (1v32, 16v17, 9v24...), so the
//! two best seeds can only meet in the final. Winners advance in pairs; winners round 1's
//! losers pair up in losers round 1, and every later winners round's losers drop into
//! the even losers rounds, uncrossed. The grand final is the winners final's winner
//! against the losers final's, followed by its bracket reset, which the same two play.
//!
//! Each stage is a week in which no team plays more than twice: winners round 1 alone,
//! then each winners round with the two losers rounds that follow it (the first losers
//! round alone), and the grand final with the losers final and the reset.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::bracket::{Outcome, Side};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BracketFormat {
    #[serde(rename = "round_of_32")]
    RoundOf32,
    #[serde(rename = "round_of_16")]
    RoundOf16,
}

impl BracketFormat {
    // How many teams it seeds; the rest are out of the tournament.
    pub fn teams(self) -> usize {
        match self {
            Self::RoundOf32 => 32,
            Self::RoundOf16 => 16,
        }
    }

    // Winners rounds up to and including the winners final.
    fn winners_rounds(self) -> usize {
        self.teams().trailing_zeros() as usize
    }

    fn losers_rounds(self) -> usize {
        2 * (self.winners_rounds() - 1)
    }

    // The stages (weeks) it's played over, the grand final's included.
    pub fn weeks(self) -> usize {
        self.winners_rounds() + 1
    }
}

// The first round's pairings, top to bottom, as 1-based seeds: (1, 32), (16, 17)...
pub fn first_round(teams: usize) -> Vec<(usize, usize)> {
    // The better seed of each match, top to bottom; it meets its mirror (teams + 1 - seed).
    let mut order = vec![1];
    while order.len() < teams / 2 {
        let size = order.len() * 2;
        // Each seed is joined by its mirror, every other pair turned round so the halves
        // alternate.
        order = order
            .iter()
            .enumerate()
            .flat_map(|(i, &s)| {
                if i % 2 == 0 {
                    [s, size + 1 - s]
                } else {
                    [size + 1 - s, s]
                }
            })
            .collect();
    }
    order.into_iter().map(|s| (s, teams + 1 - s)).collect()
}

// Where a round sits on the editor's board, in board units: a card is 12x5, rounds are 15
// apart, and the losers side sits under the winners side, one round to the right.
const ROUND_GAP: i32 = 15;
const ROW_PITCH: i32 = 6;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedMatch {
    pub side: Side,
    // W1, L3: numbered within its side, from 1, in round order.
    pub number: i32,
    pub pos_x: i32,
    pub pos_y: i32,
    // Which of the format's weeks (stages) it's played in, from 0.
    pub week: usize,
    // Winners round 1: its two seeds, 1-based (team 1 is the better seed).
    pub seeds: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedLink {
    // Indexes into the plan's matches.
    pub source: usize,
    pub outcome: Outcome,
    pub target: usize,
    pub slot: i16,
}

#[derive(Debug, Clone)]
pub struct Plan {
    pub matches: Vec<PlannedMatch>,
    pub links: Vec<PlannedLink>,
}

pub fn plan(format: BracketFormat) -> Plan {
    let teams = format.teams();
    let w_rounds = format.winners_rounds();
    let l_rounds = format.losers_rounds();
    let mut matches = Vec::new();
    // Each round's match indexes, by round (1-based; index 0 unused).
    let mut winners: Vec<Vec<usize>> = vec![Vec::new()];
    let mut losers: Vec<Vec<usize>> = vec![Vec::new()];
    let mut w_num = 0;
    let mut l_num = 0;

    let pairs = first_round(teams);
    for r in 1..=w_rounds {
        let count = teams >> r;
        let spread = 1 << (r - 1);
        let mut round = Vec::new();
        for i in 0..count {
            let seeds = (r == 1).then(|| pairs.get(i).copied()).flatten();
            w_num += 1;
            round.push(matches.len());
            matches.push(PlannedMatch {
                side: Side::Winners,
                number: w_num,
                pos_x: ROUND_GAP * (r as i32 - 1),
                pos_y: ROW_PITCH / 2 * (spread - 1) + ROW_PITCH * spread * i as i32,
                week: r - 1,
                seeds,
            });
        }
        winners.push(round);
    }

    // The losers side starts a card's gap under winners round 1.
    let losers_top = ROW_PITCH * (teams as i32 / 2) + ROW_PITCH;
    for k in 1..=l_rounds {
        // Rounds come in pairs of the same size: 1-2, 3-4...
        let pair = (k - 1) / 2;
        let count = (teams / 4) >> pair;
        let spread = 1 << pair;
        let mut round = Vec::new();
        for i in 0..count {
            l_num += 1;
            round.push(matches.len());
            matches.push(PlannedMatch {
                side: Side::Losers,
                number: l_num,
                pos_x: ROUND_GAP * k as i32,
                pos_y: losers_top + ROW_PITCH / 2 * (spread - 1) + ROW_PITCH * spread * i as i32,
                week: k / 2 + 1,
                seeds: None,
            });
        }
        losers.push(round);
    }

    // The grand final and its reset, after the losers final, halfway between the two finals.
    let w_final = &matches[winners[w_rounds][0]];
    let l_final = &matches[losers[l_rounds][0]];
    let final_y = (w_final.pos_y + l_final.pos_y) / 2;
    let final_x = ROUND_GAP * (l_rounds as i32 + 1);
    let mut finals = Vec::new();
    for x in [final_x, final_x + ROUND_GAP + 1] {
        w_num += 1;
        finals.push(matches.len());
        matches.push(PlannedMatch {
            side: Side::Winners,
            number: w_num,
            pos_x: x,
            pos_y: final_y,
            week: w_rounds,
            seeds: None,
        });
    }
    let (grand_final, reset) = (finals[0], finals[1]);

    let mut links = Vec::new();
    let mut link = |source: usize, outcome: Outcome, target: usize, slot: i16| {
        links.push(PlannedLink {
            source,
            outcome,
            target,
            slot,
        })
    };
    let slot_of = |i: usize| (i % 2 + 1) as i16;
    for r in 1..w_rounds {
        for (i, &m) in winners[r].iter().enumerate() {
            link(m, Outcome::Winner, winners[r + 1][i / 2], slot_of(i));
        }
    }
    link(winners[w_rounds][0], Outcome::Winner, grand_final, 1);
    for (i, &m) in winners[1].iter().enumerate() {
        link(m, Outcome::Loser, losers[1][i / 2], slot_of(i));
    }
    for r in 2..=w_rounds {
        for (i, &m) in winners[r].iter().enumerate() {
            link(m, Outcome::Loser, losers[2 * (r - 1)][i], 2);
        }
    }
    for k in 1..l_rounds {
        for (i, &m) in losers[k].iter().enumerate() {
            if k % 2 == 1 {
                // Meets a team dropping from the winners side.
                link(m, Outcome::Winner, losers[k + 1][i], 1);
            } else {
                link(m, Outcome::Winner, losers[k + 1][i / 2], slot_of(i));
            }
        }
    }
    link(losers[l_rounds][0], Outcome::Winner, grand_final, 2);
    link(grand_final, Outcome::Loser, reset, 1);
    link(grand_final, Outcome::Winner, reset, 2);

    Plan { matches, links }
}

// The seeding order: a host's saved order, the teams that still exist in it, and every
// other team after it in the qualifier's order. Without a saved order, the qualifier's.
pub fn order(qualifier_order: &[Uuid], saved: &[Uuid]) -> Vec<Uuid> {
    let mut out: Vec<Uuid> = saved
        .iter()
        .copied()
        .filter(|id| qualifier_order.contains(id))
        .collect();
    for id in qualifier_order {
        if !out.contains(id) {
            out.push(*id);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn first_round_is_the_standard_order() {
        let tops: Vec<usize> = first_round(32).iter().map(|p| p.0).collect();
        // The dev seed's order.
        assert_eq!(
            tops,
            [1, 16, 9, 8, 5, 12, 13, 4, 3, 14, 11, 6, 7, 10, 15, 2]
        );
        assert!(first_round(32).iter().all(|(a, b)| a + b == 33));
        assert_eq!(
            first_round(16),
            [
                (1, 16),
                (8, 9),
                (5, 12),
                (4, 13),
                (3, 14),
                (6, 11),
                (7, 10),
                (2, 15)
            ]
        );
    }

    fn counts(plan: &Plan) -> (usize, usize) {
        let w = plan
            .matches
            .iter()
            .filter(|m| m.side == Side::Winners)
            .count();
        (w, plan.matches.len() - w)
    }

    #[test]
    fn match_counts() {
        // Winners 16+8+4+2+1 and the grand final and reset; losers 8+8+4+4+2+2+1+1.
        assert_eq!(counts(&plan(BracketFormat::RoundOf32)), (33, 30));
        assert_eq!(counts(&plan(BracketFormat::RoundOf16)), (17, 14));
    }

    #[test]
    fn the_round_of_32_lays_out_like_the_seed() {
        let plan = plan(BracketFormat::RoundOf32);
        let at = |side: Side, number: i32| {
            let m = plan
                .matches
                .iter()
                .find(|m| m.side == side && m.number == number)
                .unwrap();
            (m.pos_x, m.pos_y, m.week)
        };
        assert_eq!(at(Side::Winners, 1), (0, 0, 0));
        assert_eq!(at(Side::Winners, 17), (15, 3, 1));
        assert_eq!(at(Side::Winners, 25), (30, 9, 2));
        assert_eq!(at(Side::Winners, 29), (45, 21, 3));
        assert_eq!(at(Side::Winners, 31), (60, 45, 4));
        assert_eq!(at(Side::Winners, 32), (135, 84, 5));
        assert_eq!(at(Side::Winners, 33), (151, 84, 5));
        assert_eq!(at(Side::Losers, 1), (15, 102, 1));
        assert_eq!(at(Side::Losers, 9), (30, 102, 2));
        assert_eq!(at(Side::Losers, 17), (45, 105, 2));
        assert_eq!(at(Side::Losers, 21), (60, 105, 3));
        assert_eq!(at(Side::Losers, 25), (75, 111, 3));
        assert_eq!(at(Side::Losers, 29), (105, 123, 4));
        assert_eq!(at(Side::Losers, 30), (120, 123, 5));
    }

    // Plays every match with team 1 winning: each team's losses, how many matches were
    // played, and how many each team played in each week.
    struct Played {
        losses: HashMap<usize, u32>,
        played: usize,
        per_week: HashMap<(usize, usize), u32>,
    }

    fn play(format: BracketFormat) -> Played {
        let plan = plan(format);
        let n = plan.matches.len();
        let mut slots: Vec<[Option<usize>; 2]> = vec![[None, None]; n];
        for (i, m) in plan.matches.iter().enumerate() {
            if let Some((a, b)) = m.seeds {
                slots[i] = [Some(a), Some(b)];
            }
        }
        let mut out = Played {
            losses: HashMap::new(),
            played: 0,
            per_week: HashMap::new(),
        };
        let mut done = vec![false; n];
        while let Some(i) = (0..n).find(|&i| !done[i] && slots[i].iter().all(Option::is_some)) {
            done[i] = true;
            out.played += 1;
            let [winner, loser] = slots[i].map(Option::unwrap);
            *out.losses.entry(loser).or_default() += 1;
            for team in [winner, loser] {
                *out.per_week
                    .entry((team, plan.matches[i].week))
                    .or_default() += 1;
            }
            for l in plan.links.iter().filter(|l| l.source == i) {
                let team = if l.outcome == Outcome::Winner {
                    winner
                } else {
                    loser
                };
                let slot = &mut slots[l.target][l.slot as usize - 1];
                assert!(slot.is_none(), "a slot filled twice");
                *slot = Some(team);
            }
        }
        out
    }

    #[test]
    fn every_match_gets_its_teams_and_everyone_loses_twice_but_one() {
        for format in [BracketFormat::RoundOf32, BracketFormat::RoundOf16] {
            let played = play(format);
            assert_eq!(played.played, plan(format).matches.len());
            assert_eq!(played.losses.len(), format.teams());
            assert_eq!(played.losses.values().filter(|&&l| l < 2).count(), 1);
        }
    }

    #[test]
    fn links_are_one_per_outcome_and_slot() {
        let plan = plan(BracketFormat::RoundOf32);
        let outcomes: HashSet<_> = plan.links.iter().map(|l| (l.source, l.outcome)).collect();
        let slots: HashSet<_> = plan.links.iter().map(|l| (l.target, l.slot)).collect();
        assert_eq!(outcomes.len(), plan.links.len());
        assert_eq!(slots.len(), plan.links.len());
        // Every slot but winners round 1's is fed by a link.
        assert_eq!(plan.links.len(), 2 * (plan.matches.len() - 16));
    }

    #[test]
    fn no_team_plays_more_than_twice_a_week() {
        for format in [BracketFormat::RoundOf32, BracketFormat::RoundOf16] {
            let last = format.weeks() - 1;
            for (&(_, week), &count) in &play(format).per_week {
                // The grand final's week has the losers final, the grand final and its reset.
                assert!(count <= if week == last { 3 } else { 2 });
            }
        }
    }

    #[test]
    fn order_keeps_the_saved_order_and_adds_the_rest() {
        let ids: Vec<Uuid> = (1..=5).map(Uuid::from_u128).collect();
        assert_eq!(order(&ids, &[]), ids);
        let gone = Uuid::from_u128(99);
        assert_eq!(
            order(&ids, &[ids[3], gone, ids[0]]),
            [ids[3], ids[0], ids[1], ids[2], ids[4]]
        );
    }
}
