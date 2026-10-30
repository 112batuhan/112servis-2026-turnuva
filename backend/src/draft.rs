//! A stage's pick and ban order (`stages.draft_order`), written by hosts as steps
//! joined by dashes: `p1-p2-b2-b1-p1-*`. `p` is a map picked (and played), `b` a ban,
//! and the figure is the team: 1 is the team that marks the first ban or pick on the
//! referee page (the roll decides it), 2 the other. A closing `*` has the teams take
//! turns picking from there, each pick going to the team that didn't make the one
//! before. The referee page and the room's bot read whose turn it is from it.

use serde::Serialize;
use uuid::Uuid;

// What a stage that has no order of its own plays: two bans, then picks in turn from
// the team that didn't ban first.
pub const DEFAULT_ORDER: &str = "b1-b2-p2-*";

// The most steps an order may spell out.
pub const MAX_STEPS: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    // Reads an order, ignoring case and spaces. The error says what's wrong, for the
    // host who typed it.
    pub fn parse(text: &str) -> Result<Self, &'static str> {
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
            return Err("the order needs at least one ban or pick");
        }
        if tokens.len() > MAX_STEPS {
            return Err("the order has more than 40 steps");
        }
        let steps = tokens
            .iter()
            .map(|t| {
                let mut chars = t.chars();
                let kind = match chars.next() {
                    Some('p') => StepKind::Pick,
                    Some('b') => StepKind::Ban,
                    _ => return Err("each step is p1, p2, b1 or b2, and * only at the end"),
                };
                let team = match (chars.next(), chars.next()) {
                    (Some('1'), None) => 1,
                    (Some('2'), None) => 2,
                    _ => return Err("each step is p1, p2, b1 or b2, and * only at the end"),
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

// What comes next after the bans and picks so far, given as the team that made each
// in the order they were marked:
// the step at that count, its team read from who made the first one. `chosen` is the
// team the referee said goes next, which stands in for the rules until the next mark.
// None once the order has run out.
pub fn next_turn(
    order: &DraftOrder,
    marks: &[Uuid],
    teams: [Uuid; 2],
    chosen: Option<Uuid>,
) -> Option<Turn> {
    let other = |id: Uuid| teams.into_iter().find(|&t| t != id);
    let n = marks.len();
    let (kind, by_rule) = match order.steps.get(n) {
        Some(step) => {
            let team = marks.first().and_then(|&first| {
                // The first mark was made by the first step's team.
                let one = if order.steps[0].team == 1 {
                    Some(first)
                } else {
                    other(first)
                };
                if step.team == 1 {
                    one
                } else {
                    one.and_then(other)
                }
            });
            (step.kind, team)
        }
        None if order.alternate => (StepKind::Pick, marks.last().and_then(|&last| other(last))),
        None => return None,
    };
    let team_id = chosen.filter(|c| teams.contains(c)).or(by_rule);
    Some(Turn { kind, team_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: Uuid = Uuid::from_u128(1);
    const B: Uuid = Uuid::from_u128(2);

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
            next_turn(&order, &[], teams, None),
            Some(Turn {
                kind: Pick,
                team_id: None
            })
        );
        // Red (B) won the roll and picked first: it's 1.
        let mut done = vec![B];
        let next = |done: &[Uuid]| next_turn(&order, done, teams, None).unwrap();
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
            next_turn(&order, &done, teams, Some(B)),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        // An order without * runs out.
        let short = DraftOrder::parse("b2-p1").unwrap();
        let done = [A];
        assert_eq!(
            next_turn(&short, &done, teams, None),
            Some(Turn {
                kind: Pick,
                team_id: Some(B)
            })
        );
        assert_eq!(next_turn(&short, &[A, B], teams, None), None);
    }

    #[test]
    fn the_default_is_the_old_rule() {
        let order = DraftOrder::default();
        let done = [A, B];
        assert_eq!(
            next_turn(&order, &done, [A, B], None),
            Some(Turn {
                kind: StepKind::Pick,
                team_id: Some(B)
            })
        );
    }
}
