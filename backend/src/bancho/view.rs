//! A match's room as every page shows it: the referee page, the public match page and
//! the bots page all read this one view, pushed over the live socket as it changes.

use std::collections::{HashMap, HashSet};

use serde::Serialize;
use uuid::Uuid;

use super::{
    autoref::AutorefView,
    context::MatchContext,
    room::{MapScore, RoomState},
    tally::{
        entry_of, room_results, row_of, standing_of, tally, turn_of, Finished, Standing,
        TallyInput, TeamTotal,
    },
};
use crate::{draft::Turn, osu_api::matches::MatchGame};

// Where a room's bot stands with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomAccess {
    // The bot is in its channel: it logs it and runs it.
    In,
    // The bot is connected and on its way in.
    Joining,
    // Bancho wouldn't let the bot in: a room made elsewhere whose maker hasn't added it
    // as a referee.
    Refused,
    // The bot isn't connected to Bancho.
    Offline,
    // No bot runs the room.
    NoBot,
    // The room was closed.
    Closed,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoomBadge {
    pub mp_id: i64,
    // The bot that runs it, by osu! username.
    pub bot: Option<String>,
    pub access: RoomAccess,
}

#[derive(Debug, Clone, Serialize)]
pub struct FinishedView {
    // "<source>:<room>:<n>"; "Sayma" is keyed by it.
    pub key: String,
    pub mp_id: i64,
    pub beatmap_id: Option<i64>,
    pub beatmap_name: Option<String>,
    pub room_mods: String,
    pub freemod: bool,
    pub scores: Vec<MapScore>,
    // The pool map it was, or None for a map outside the pool (a warmup, a mistake).
    pub map_id: Option<Uuid>,
    // Each team's summed score on it, best first.
    pub totals: Vec<TeamTotal>,
    // Bracket matches: the team that won it, or None for a tie.
    pub winner_id: Option<Uuid>,
    // Whether it counts towards the score (not left out with "Sayma").
    pub counted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TallyView {
    pub team1: i32,
    pub team2: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LiveRoom {
    // Every room of the match, oldest first, with its bot.
    pub rooms: Vec<RoomBadge>,
    // The latest room, where the match is played, as BanchoBot has described it.
    pub state: RoomState,
    // The pool map the latest room has on.
    pub current_map_id: Option<Uuid>,
    // Every room's finished maps, oldest room first.
    pub finished: Vec<FinishedView>,
    // Bracket matches with both teams: the map wins counted from the rooms.
    pub tally: Option<TallyView>,
    // Whether the site has that score and the maps behind it.
    pub score_saved: bool,
    // Bracket matches: the next ban or pick by the stage's order, while there's one to
    // make. The pages read it from the bans and picks themselves (lib/draftOrder.ts);
    // the bot says it with the score.
    #[serde(skip)]
    pub next_turn: Option<Turn>,
    // Whether the results come from osu!'s record of the rooms up to its last map.
    pub osu_synced: bool,
    // Where the auto referee stands (staff pages show it); None when the bots don't run
    // the match.
    pub autoref: Option<AutorefView>,
}

pub struct ViewRoom<'a> {
    pub mp_id: i64,
    pub state: &'a RoomState,
    pub bot: Option<String>,
    pub access: RoomAccess,
}

// The match's finished maps, from each room's chat, with osu!'s record in their place
// once the referee synced it.
pub fn finished_of(
    rooms: &[ViewRoom],
    games: Option<&HashMap<i64, Vec<MatchGame>>>,
) -> Vec<Finished> {
    rooms
        .iter()
        .flat_map(|r| {
            let games = games.and_then(|g| g.get(&r.mp_id)).map(Vec::as_slice);
            room_results(r.mp_id, &r.state.results, games)
        })
        .collect()
}

pub fn build(
    ctx: &MatchContext,
    rooms: &[ViewRoom],
    games: Option<&HashMap<i64, Vec<MatchGame>>>,
) -> LiveRoom {
    let finished = finished_of(rooms, games);
    let picked: HashSet<Uuid> = ctx.picks.iter().map(|p| p.map_id).collect();
    let latest = rooms.last();
    let state = latest.map(|r| r.state.clone()).unwrap_or_default();
    let current_map_id = entry_of(
        &ctx.entries,
        state.beatmap_id,
        &super::room::mod_acronyms(state.mods.as_deref().unwrap_or_default()),
        super::room::is_freemod(state.mods.as_deref()),
        &picked,
    )
    .filter(|_| state.beatmap_id.is_some())
    .map(|e| e.map_id);

    let tallied = ctx
        .sides
        .as_ref()
        .filter(|_| ctx.is_bracket())
        .map(|[a, b]| {
            tally(&TallyInput {
                finished: &finished,
                entries: &ctx.entries,
                picks: &ctx.picks,
                roster: &ctx.roster,
                ignored: &ctx.ignored,
                manual: &ctx.manual,
                team1_id: a.id,
                team2_id: b.id,
            })
        });
    let score_saved = tallied.as_ref().is_some_and(|t| {
        super::tally::score_key(Some(t.team1), Some(t.team2), &t.maps) == ctx.saved_key
    });

    // Nobody picks at the tiebreaker or once a team has won.
    let drafting = match (&tallied, ctx.best_of) {
        (Some(t), Some(best_of)) => !matches!(
            standing_of(best_of, t.team1, t.team2),
            Standing::Tiebreaker | Standing::Won
        ),
        _ => true,
    };
    let next_turn = ctx
        .sides
        .as_ref()
        .filter(|_| drafting && !ctx.concluded())
        .and_then(|[a, b]| {
            turn_of(
                &ctx.draft_order,
                &ctx.bans,
                &ctx.picks,
                [a.id, b.id],
                ctx.settings.next_pick_team_id,
            )
        });

    LiveRoom {
        rooms: rooms
            .iter()
            .map(|r| RoomBadge {
                mp_id: r.mp_id,
                bot: r.bot.clone(),
                access: r.access,
            })
            .collect(),
        current_map_id,
        finished: finished
            .iter()
            .map(|f| {
                let row = row_of(f, &ctx.entries, &picked, &ctx.roster);
                FinishedView {
                    key: f.key.clone(),
                    mp_id: f.mp_id,
                    beatmap_id: f.result.beatmap_id,
                    beatmap_name: f.result.beatmap_name.clone(),
                    room_mods: f.result.room_mods.clone(),
                    freemod: f.result.freemod,
                    scores: f.result.scores.clone(),
                    map_id: row.as_ref().map(|r| r.map_id),
                    totals: row
                        .as_ref()
                        .map(|r| r.totals.clone())
                        .unwrap_or_else(|| super::tally::team_totals(&f.result, &ctx.roster)),
                    winner_id: row
                        .as_ref()
                        .and_then(|r| r.winner_id)
                        .filter(|_| ctx.is_bracket()),
                    counted: !ctx.ignored.contains(&f.key),
                }
            })
            .collect(),
        state,
        tally: tallied.as_ref().map(|t| TallyView {
            team1: t.team1,
            team2: t.team2,
        }),
        score_saved,
        next_turn,
        osu_synced: games.is_some(),
        autoref: None,
    }
}
