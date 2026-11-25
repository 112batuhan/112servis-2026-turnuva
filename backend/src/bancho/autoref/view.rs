//! The auto referee as the referee page shows it.

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::draft::StepKind;

use super::{AutoRef, CountdownKind, Phase, Snapshot, GRACE};

impl AutoRef {
    pub fn view(&self, s: &Snapshot, enabled: bool) -> AutorefView {
        let (team_id, step, map_id, deadline) = match &self.phase {
            Phase::Waiting => (
                None,
                None,
                None,
                s.starts_at.filter(|&t| s.now < t).or_else(|| {
                    self.countdown_deadline(CountdownKind::NoShow)
                        .or_else(|| s.deadline().map(|d| d + GRACE))
                }),
            ),
            Phase::Warmup { team, until, .. } => {
                (s.teams.get(*team).map(|t| t.id), None, None, Some(*until))
            }
            Phase::Roll { until, .. } => (None, None, None, Some(*until)),
            Phase::Draft {
                team_id,
                kind,
                until,
                ..
            } => (Some(*team_id), Some(*kind), None, Some(*until)),
            Phase::Ready { map_id, until, .. } => (None, None, Some(*map_id), Some(*until)),
            Phase::Playing { map_id, .. } | Phase::Scoring { map_id, .. } => {
                (None, None, Some(*map_id), None)
            }
            Phase::Next | Phase::Done => (None, None, None, None),
        };
        let (warmup_beatmap_id, warmup_playing, warmup_skipped) = match &self.phase {
            Phase::Warmup {
                beatmap,
                played,
                skipped,
                ..
            } => (
                *beatmap,
                *played || s.room.playing,
                s.teams
                    .iter()
                    .zip(skipped)
                    .filter(|(_, &skip)| skip)
                    .map(|(t, _)| t.id)
                    .collect(),
            ),
            _ => (None, false, Vec::new()),
        };
        let rolls = match &self.phase {
            Phase::Roll { rolls, .. } => s
                .teams
                .iter()
                .zip(rolls)
                .filter_map(|(t, r)| {
                    r.map(|roll| RollView {
                        team_id: t.id,
                        roll,
                    })
                })
                .collect(),
            _ => Vec::new(),
        };
        AutorefView {
            enabled,
            phase: phase_key(&self.phase),
            team_id,
            step,
            map_id,
            deadline,
            warmup_beatmap_id,
            warmup_playing,
            warmup_skipped,
            rolls,
            teams: s
                .teams
                .iter()
                .map(|t| TeamUsesView {
                    team_id: t.id,
                    aborts_left: self.aborts_left(t.id),
                })
                .collect(),
            stand_ins: s.stand_ins,
        }
    }
}

fn phase_key(phase: &Phase) -> &'static str {
    match phase {
        Phase::Waiting => "waiting",
        Phase::Warmup { .. } => "warmup",
        Phase::Roll { .. } => "roll",
        Phase::Draft { .. } => "draft",
        Phase::Ready { .. } => "ready",
        Phase::Playing { .. } => "playing",
        Phase::Scoring { .. } => "scoring",
        Phase::Next => "next",
        Phase::Done => "done",
    }
}

pub(super) fn phase_name(phase: &Phase) -> &'static str {
    match phase {
        Phase::Waiting => "oyuncular bekleniyor",
        Phase::Warmup { .. } => "warmup",
        Phase::Roll { .. } => "roll",
        Phase::Draft { .. } => "ban/pick",
        Phase::Ready { .. } => "map hazırlığı",
        Phase::Playing { .. } => "map oynanıyor",
        Phase::Scoring { .. } => "skorlar",
        Phase::Next => "sıradaki adım",
        Phase::Done => "maç bitti",
    }
}

// The auto referee as the referee page shows it.
#[derive(Debug, Clone, Serialize)]
pub struct AutorefView {
    // The match is in auto mode.
    pub enabled: bool,
    pub phase: &'static str,
    // Warmup: the team choosing its warmup; draft: the team to ban or pick.
    pub team_id: Option<Uuid>,
    pub step: Option<StepKind>,
    // Ready, playing, scoring: the pool map on.
    pub map_id: Option<Uuid>,
    // When the phase's timer runs out (before the match: its time, then the no-show
    // cutoff).
    #[serde(with = "time::serde::rfc3339::option")]
    pub deadline: Option<OffsetDateTime>,
    // Warmup: the beatmap the team chose on the room page (the bot put it on), whether
    // it's being played, and the teams that said they'd skip theirs ahead of their turn.
    pub warmup_beatmap_id: Option<i64>,
    pub warmup_playing: bool,
    pub warmup_skipped: Vec<Uuid>,
    // Roll: each team's first !roll so far.
    pub rolls: Vec<RollView>,
    pub teams: Vec<TeamUsesView>,
    // DEV_LOGIN: the dummy accounts (negative osu! ids) count as in the room.
    pub stand_ins: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollView {
    pub team_id: Uuid,
    pub roll: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TeamUsesView {
    pub team_id: Uuid,
    pub aborts_left: u8,
}
