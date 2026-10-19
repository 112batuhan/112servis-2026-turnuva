//! Potential matches: the pairings an undecided bracket match may still turn out to be.
//!
//! Some weeks a team plays twice, the second match against whoever comes out of another
//! match of the same week, so the players have to agree on its time before they know
//! who they'll meet. When an unconcluded match's empty slot waits on an unconcluded
//! match of the same stage whose two teams are known, every team that can reach each
//! slot is paired with every one that can reach the other, and each pairing becomes a
//! potential match (db::potential::sync keeps them in step with the bracket). A slot
//! that waits on another stage, or on a match still missing a team, makes no pairings:
//! those would be a third match in the week.

use std::collections::HashSet;

use uuid::Uuid;

use crate::db::bracket::LinkRow;

// A bracket match as the pairings read it.
#[derive(Debug, Clone, Copy)]
pub struct Node {
    pub id: Uuid,
    pub stage_id: Option<Uuid>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub concluded: bool,
}

// One pairing a bracket match may turn out to be, team 1 in its slot 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pairing {
    pub match_id: Uuid,
    pub team1_id: Uuid,
    pub team2_id: Uuid,
}

// Whether two pairs are the same two teams, in either order.
pub fn same_teams(a: (Uuid, Uuid), b: (Uuid, Uuid)) -> bool {
    a == b || a == (b.1, b.0)
}

// The teams that can still end up in slot `slot` of `target`: the one in it, or both
// teams of the match of the same stage it waits on. None when that isn't known yet.
fn candidates(nodes: &[Node], links: &[LinkRow], target: &Node, slot: i16) -> Option<Vec<Uuid>> {
    let team = if slot == 1 {
        target.team1_id
    } else {
        target.team2_id
    };
    if let Some(team) = team {
        return Some(vec![team]);
    }
    let link = links
        .iter()
        .find(|l| l.target_id == target.id && l.slot == slot)?;
    let source = nodes.iter().find(|n| n.id == link.source_id)?;
    if source.concluded || source.stage_id.is_none() || source.stage_id != target.stage_id {
        return None;
    }
    Some(vec![source.team1_id?, source.team2_id?])
}

// Every pairing the bracket's undecided matches may turn out to be. A match whose two
// slots wait on the same match (a grand final's reset) has the same two teams either
// way, so it gets one pairing.
pub fn pairings(nodes: &[Node], links: &[LinkRow]) -> Vec<Pairing> {
    let mut out = Vec::new();
    for target in nodes {
        if target.concluded || target.stage_id.is_none() {
            continue;
        }
        if target.team1_id.is_some() && target.team2_id.is_some() {
            continue;
        }
        let (Some(first), Some(second)) = (
            candidates(nodes, links, target, 1),
            candidates(nodes, links, target, 2),
        ) else {
            continue;
        };
        let mut seen: HashSet<(Uuid, Uuid)> = HashSet::new();
        for &a in &first {
            for &b in &second {
                if a == b || seen.contains(&(a, b)) || seen.contains(&(b, a)) {
                    continue;
                }
                seen.insert((a, b));
                out.push(Pairing {
                    match_id: target.id,
                    team1_id: a,
                    team2_id: b,
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::bracket::Outcome;

    fn id(n: u128) -> Uuid {
        Uuid::from_u128(n)
    }

    fn node(
        n: u128,
        stage: Option<u128>,
        teams: (Option<u128>, Option<u128>),
        concluded: bool,
    ) -> Node {
        Node {
            id: id(n),
            stage_id: stage.map(id),
            team1_id: teams.0.map(id),
            team2_id: teams.1.map(id),
            concluded,
        }
    }

    fn link(source: u128, outcome: Outcome, target: u128, slot: i16) -> LinkRow {
        LinkRow {
            source_id: id(source),
            outcome,
            target_id: id(target),
            slot,
        }
    }

    fn teams(p: &[Pairing]) -> Vec<(u128, u128)> {
        p.iter()
            .map(|p| (p.team1_id.as_u128(), p.team2_id.as_u128()))
            .collect()
    }

    const S: u128 = 900;
    const LATER: u128 = 901;

    #[test]
    fn two_matches_of_the_week_feeding_one_make_four_pairings() {
        let nodes = [
            node(1, Some(S), (Some(11), Some(12)), false),
            node(2, Some(S), (Some(21), Some(22)), false),
            node(3, Some(S), (None, None), false),
        ];
        let links = [
            link(1, Outcome::Winner, 3, 1),
            link(2, Outcome::Winner, 3, 2),
        ];
        let p = pairings(&nodes, &links);
        assert!(p.iter().all(|p| p.match_id == id(3)));
        assert_eq!(teams(&p), [(11, 21), (11, 22), (12, 21), (12, 22)]);
    }

    #[test]
    fn a_known_team_meets_both_teams_of_the_other_slot() {
        let nodes = [
            node(1, Some(S), (Some(11), Some(12)), false),
            node(3, Some(S), (Some(30), None), false),
        ];
        let links = [link(1, Outcome::Loser, 3, 2)];
        assert_eq!(teams(&pairings(&nodes, &links)), [(30, 11), (30, 12)]);
    }

    #[test]
    fn a_slot_fed_from_another_stage_or_an_unknown_team_makes_none() {
        let links = [
            link(1, Outcome::Winner, 3, 1),
            link(2, Outcome::Winner, 3, 2),
        ];
        let other_stage = [
            node(1, Some(S), (Some(11), Some(12)), false),
            node(2, Some(LATER), (Some(21), Some(22)), false),
            node(3, Some(LATER), (None, None), false),
        ];
        assert!(pairings(&other_stage, &links).is_empty());
        let missing_team = [
            node(1, Some(S), (Some(11), Some(12)), false),
            node(2, Some(S), (Some(21), None), false),
            node(3, Some(S), (None, None), false),
        ];
        assert!(pairings(&missing_team, &links).is_empty());
        let no_stage = [
            node(1, None, (Some(11), Some(12)), false),
            node(2, None, (Some(21), Some(22)), false),
            node(3, None, (None, None), false),
        ];
        assert!(pairings(&no_stage, &links).is_empty());
    }

    #[test]
    fn a_decided_or_concluded_match_has_none() {
        let links = [link(1, Outcome::Winner, 3, 1)];
        let decided = [
            node(1, Some(S), (Some(11), Some(12)), false),
            node(3, Some(S), (Some(11), Some(30)), false),
        ];
        assert!(pairings(&decided, &links).is_empty());
        // A concluded source whose team a host took back out leaves the slot open.
        let concluded_source = [
            node(1, Some(S), (Some(11), Some(12)), true),
            node(3, Some(S), (None, Some(30)), false),
        ];
        assert!(pairings(&concluded_source, &links).is_empty());
    }

    #[test]
    fn a_reset_fed_twice_by_one_match_is_one_pairing() {
        let nodes = [
            node(1, Some(S), (Some(11), Some(12)), false),
            node(3, Some(S), (None, None), false),
        ];
        let links = [
            link(1, Outcome::Loser, 3, 1),
            link(1, Outcome::Winner, 3, 2),
        ];
        assert_eq!(teams(&pairings(&nodes, &links)), [(11, 12)]);
    }

    #[test]
    fn same_teams_ignores_the_order() {
        assert!(same_teams((id(1), id(2)), (id(2), id(1))));
        assert!(same_teams((id(1), id(2)), (id(1), id(2))));
        assert!(!same_teams((id(1), id(2)), (id(1), id(3))));
    }
}
