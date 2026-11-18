//! A stage's pick and ban order (`stages.draft_order`), written by hosts as steps
//! joined by dashes: `p1-p2-b2-b1-p1-*`. `p` is a map picked (and played), `b` a ban,
//! and the figure is the team: 1 is the team that marks the first ban or pick on the
//! referee page (the roll decides it), 2 the other. A closing `*` has the teams take
//! turns picking from there, each pick going to the team that didn't make the one
//! before. The referee page and the room's bot read whose turn it is from it.
//!
//! A turn can also run out on the auto referee's clock (`match_missed_turns`): a missed
//! ban takes its step with no map banned, and a missed pick passes that step to the
//! other team.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::PoolError;

// What a stage that has no order of its own plays: two bans, then picks in turn from
// the team that didn't ban first.
pub const DEFAULT_ORDER: &str = "b1-b2-p2-*";

// The most steps an order may spell out.
pub const MAX_STEPS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "draft_step_kind", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum StepKind {
    Ban,
    Pick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Step {
    pub kind: StepKind,
    // 1 or 2.
    pub team: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftOrder {
    pub steps: Vec<Step>,
    // A closing `*`: picks take turns after the last step.
    pub alternate: bool,
}

impl Default for DraftOrder {
    fn default() -> Self {
        Self::parse(DEFAULT_ORDER).expect("the default order parses")
    }
}

impl DraftOrder {
    // Reads an order, ignoring case and spaces. The error says what's wrong, for the host
    // who typed it.
    pub fn parse(text: &str) -> Result<Self, PoolError> {
        let text: String = text
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_lowercase();
        let tokens: Vec<&str> = text.split('-').collect();
        let (alternate, tokens) = match tokens.split_last() {
            Some((&"*", rest)) => (true, rest),
            _ => (false, &tokens[..]),
        };
        if tokens.is_empty() || tokens == [""] {
            return Err(PoolError::DraftOrderEmpty);
        }
        if tokens.len() > MAX_STEPS {
            return Err(PoolError::DraftOrderTooLong);
        }
        let steps = tokens
            .iter()
            .map(|t| {
                let mut chars = t.chars();
                let kind = match chars.next() {
                    Some('p') => StepKind::Pick,
                    Some('b') => StepKind::Ban,
                    _ => return Err(PoolError::DraftOrderStep),
                };
                let team = match (chars.next(), chars.next()) {
                    (Some('1'), None) => 1,
                    (Some('2'), None) => 2,
                    _ => return Err(PoolError::DraftOrderStep),
                };
                Ok(Step { kind, team })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { steps, alternate })
    }

    // How it's stored: lower case, no spaces.
    pub fn format(&self) -> String {
        let mut parts: Vec<String> = self
            .steps
            .iter()
            .map(|s| {
                let kind = match s.kind {
                    StepKind::Ban => 'b',
                    StepKind::Pick => 'p',
                };
                format!("{kind}{}", s.team)
            })
            .collect();
        if self.alternate {
            parts.push("*".into());
        }
        parts.join("-")
    }

    // A stage's order, or the default when it has none (a stored one always parses).
    pub fn of_stage(stored: Option<&str>) -> Self {
        stored.and_then(|s| Self::parse(s).ok()).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Turn {
    pub kind: StepKind,
    // None until the first ban or pick says which team is 1.
    pub team_id: Option<Uuid>,
}

// A turn the draft has had so far: the team whose it was, and whether it was a pick
// that ran out and so passed to the other team. That takes no step of the order; a ban
// that ran out takes its step like one made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Taken {
    pub team_id: Uuid,
    pub passed: bool,
}

impl Taken {
    pub fn made(team_id: Uuid) -> Self {
        Self {
            team_id,
            passed: false,
        }
    }
}

// What comes next after the turns so far, in the order they were taken: the step at the
// count of those that took one, its team read from whose the first turn was. A pick that
// just passed goes to the other team. `chosen` is the team the referee said goes next,
// which stands in for the rules until the next mark. None once the order has run out.
pub fn next_turn(
    order: &DraftOrder,
    taken: &[Taken],
    teams: [Uuid; 2],
    chosen: Option<Uuid>,
) -> Option<Turn> {
    let other = |id: Uuid| teams.into_iter().find(|&t| t != id);
    let made: Vec<Uuid> = taken
        .iter()
        .filter(|t| !t.passed)
        .map(|t| t.team_id)
        .collect();
    let (kind, by_rule) = match order.steps.get(made.len()) {
        Some(step) => {
            let team = taken.first().and_then(|first| {
                // The first turn was the first step's team's.
                let one = if order.steps[0].team == 1 {
                    Some(first.team_id)
                } else {
                    other(first.team_id)
                };
                if step.team == 1 {
                    one
                } else {
                    one.and_then(other)
                }
            });
            (step.kind, team)
        }
        None if order.alternate => (StepKind::Pick, made.last().and_then(|&last| other(last))),
        None => return None,
    };
    let passed = taken
        .last()
        .filter(|t| t.passed)
        .and_then(|t| other(t.team_id));
    let team_id = chosen.filter(|c| teams.contains(c)).or(passed).or(by_rule);
    Some(Turn { kind, team_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Uuid = Uuid::from_u128(1);
    const B: Uuid = Uuid::from_u128(2);

    // The next turn after these bans and picks, none of them missed.
    fn turn_after(
        order: &DraftOrder,
        done: &[Uuid],
        teams: [Uuid; 2],
        chosen: Option<Uuid>,
    ) -> Option<Turn> {
        let taken: Vec<Taken> = done.iter().map(|&t| Taken::made(t)).collect();
        next_turn(order, &taken, teams, chosen)
    }

    #[test]
    fn parses_and_formats() {
        let order = DraftOrder::parse(" P1 - p2-b2-B1-p1-* ").unwrap();
        assert_eq!(order.steps.len(), 5);
        assert!(order.alternate);
        assert_eq!(order.format(), "p1-p2-b2-b1-p1-*");
        assert_eq!(DraftOrder::parse("b1-b2").unwrap().format(), "b1-b2");
        for bad in ["", "*", "p1-*-p2", "p3", "x1", "p1--p2", "p12", "p1-"] {
            assert!(DraftOrder::parse(bad).is_err(), "{bad}");
        }
        assert_eq!(DraftOrder::of_stage(None).format(), DEFAULT_ORDER);
    }

    #[test]
    fn follows_the_order() {
        use StepKind::{Ban, Pick};
        let order = DraftOrder::parse("p1-p2-b2-b1-p1-*").unwrap();
        let teams = [A, B];
        // Nobody has marked anything, so the roll hasn't said who 1 is.
        assert_eq!(
            turn_after(&order, &[], teams, None),
            Some(Turn {
                kind: Pick,
                team_id: None
            })
        );
        // Red (B) won the roll and picked first: it's 1.
        let mut done = vec![B];
        let next = |done: &[Uuid]| turn_after(&order, done, teams, None).unwrap();
        assert_eq!(
            next(&done),
            Turn {
                kind: Pick,
                team_id: Some(A)
            }
        );
        done.push(A);
        assert_eq!(
            next(&done),
            Turn {
                kind: Ban,
                team_id: Some(A)
            }
        );
        done.push(A);
        assert_eq!(
            next(&done),
            Turn {
                kind: Ban,
                team_id: Some(B)
            }
        );
        done.push(B);
        assert_eq!(
            next(&done),
            Turn {
                kind: Pick,
                team_id: Some(B)
            }
        );
        done.push(B);
        // Then the picks take turns, following who actually picked.
        assert_eq!(
            next(&done),
            Turn {
                kind: Pick,
                team_id: Some(A)
            }
        );
        done.push(B);
        assert_eq!(
            next(&done),
            Turn {
                kind: Pick,
                team_id: Some(A)
            }
        );
        // The referee's say stands in for the rules.
        assert_eq!(
            turn_after(&order, &done, teams, Some(B)),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        // An order without * runs out.
        let short = DraftOrder::parse("b2-p1").unwrap();
        let done = [A];
        assert_eq!(
            turn_after(&short, &done, teams, None),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        assert_eq!(turn_after(&short, &[A, B], teams, None), None);
    }

    #[test]
    fn the_default_is_the_old_rule() {
        let order = DraftOrder::default();
        let done = [A, B];
        assert_eq!(
            turn_after(&order, &done, [A, B], None),
            Some(Turn {
                kind: StepKind::Pick,
                team_id: Some(B)
            })
        );
    }

    #[test]
    fn missed_turns() {
        use StepKind::{Ban, Pick};
        let order = DraftOrder::parse("p1-p2-b2-b1-p1-*").unwrap();
        let teams = [A, B];
        let missed = Taken {
            team_id: A,
            passed: true,
        };
        // A won the roll and let its first pick run out: B picks it instead, and A is
        // still 1.
        assert_eq!(
            next_turn(&order, &[missed], teams, None),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        // The referee's say still stands in for the pass.
        assert_eq!(
            next_turn(&order, &[missed], teams, Some(A))
                .unwrap()
                .team_id,
            Some(A)
        );
        let taken = [missed, Taken::made(B)];
        assert_eq!(
            next_turn(&order, &taken, teams, None),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        // A ban that ran out takes its step.
        let taken = [missed, Taken::made(B), Taken::made(B), Taken::made(B)];
        assert_eq!(
            next_turn(&order, &taken, teams, None),
            Some(Turn {
                kind: Ban,
                team_id: Some(A)
            })
        );
        // Taking turns, a pick passed to B leaves the next one to A.
        let order = DraftOrder::parse("p1-*").unwrap();
        let taken = [
            Taken::made(A),
            Taken {
                team_id: B,
                passed: true,
            },
            Taken::made(A),
        ];
        assert_eq!(
            next_turn(&order, &taken, teams, None),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        // The first turn missed, a ban: it was 1's all the same.
        let order = DraftOrder::parse("b1-b2-p2-*").unwrap();
        assert_eq!(
            next_turn(&order, &[Taken::made(B)], teams, None),
            Some(Turn {
                kind: Ban,
                team_id: Some(A)
            })
        );
    }
}
