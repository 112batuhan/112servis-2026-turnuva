//! A match's rooms read without the manager: from their saved chat (a match the bots
//! don't run, or a lobby's results saved late) and from osu!'s record of them.

use std::collections::HashMap;

use uuid::Uuid;

use crate::bancho::{
    context,
    room::{read_lobby, RoomState},
    tally::lobby_rows,
    view::{self, finished_of, LiveRoom, RoomAccess, ViewRoom},
};
use crate::{
    db::{self, matches::MatchKind},
    error::AppError,
    osu_api,
    osu_api::matches::MatchGame,
    AppState,
};

use super::is_bancho_bot;
use super::saving::save_lobby;

// Saves the results of the concluded lobbies that have none, read from their saved chat:
// the bots only save a lobby's maps while they run it. Run at startup and after a dev
// reseed.
pub async fn backfill_lobbies(app: &AppState) -> Result<(), AppError> {
    for id in db::stats::lobbies_without_results(&app.db).await? {
        let Some(ctx) = context::load(&app.db, id).await? else {
            continue;
        };
        let Some(m) = db::matches::find_match(&app.db, id).await? else {
            continue;
        };
        let states = chat_states(app, &m.mp_ids).await?;
        let rooms: Vec<ViewRoom> = states
            .iter()
            .map(|(mp_id, state)| ViewRoom {
                mp_id: *mp_id,
                state,
                bot: None,
                access: RoomAccess::Closed,
            })
            .collect();
        let rows = lobby_rows(
            &finished_of(&rooms, None),
            &ctx.entries,
            &ctx.roster,
            &ctx.ignored,
        );
        if !rows.is_empty() {
            save_lobby(app, id, &rows).await?;
        }
    }
    Ok(())
}

// Each room as its saved chat describes it.
async fn chat_states(app: &AppState, mp_ids: &[i64]) -> Result<Vec<(i64, RoomState)>, AppError> {
    let mut states = Vec::new();
    for &mp_id in mp_ids {
        let lines = db::chat::list_messages(&app.db, mp_id).await?;
        let state = read_lobby(
            lines
                .iter()
                .filter(|l| is_bancho_bot(&l.sender) && !l.action)
                .map(|l| (l.text.as_str(), l.sent_at)),
        );
        states.push((mp_id, state));
    }
    Ok(states)
}

// osu!'s record of each of the rooms; a room osu! has no record of is left out.
pub async fn read_osu_games(
    app: &AppState,
    rooms: &[i64],
) -> Result<HashMap<i64, Vec<MatchGame>>, AppError> {
    let token =
        osu_api::token::app_token(&app.osu_client, &app.oauth_http, &app.osu_app_token).await?;
    let mut games = HashMap::new();
    for &mp_id in rooms {
        match osu_api::matches::fetch_games(&app.http_client, &token, mp_id).await {
            Ok(g) => {
                games.insert(mp_id, g);
            }
            Err(AppError::OsuApiRequest(err)) if osu_api::matches::refused(&err) => {
                tracing::warn!(error = %err, mp_id, "osu! has no readable record of the room");
            }
            Err(err) => return Err(err),
        }
    }
    Ok(games)
}

// A match the bots don't run (it's over, or it has no room yet), read from its saved
// chat for the pages.
pub async fn static_view(app: &AppState, match_id: Uuid) -> Result<Option<LiveRoom>, AppError> {
    let Some(ctx) = context::load(&app.db, match_id).await? else {
        return Ok(None);
    };
    let Some(m) = db::matches::find_match(&app.db, match_id).await? else {
        return Ok(None);
    };
    if m.mp_ids.is_empty() {
        return Ok(None);
    }
    let states = chat_states(app, &m.mp_ids).await?;
    let rooms: Vec<ViewRoom> = states
        .iter()
        .map(|(mp_id, state)| ViewRoom {
            mp_id: *mp_id,
            state,
            bot: None,
            access: if ctx.kind == MatchKind::Qualifier || ctx.concluded() {
                RoomAccess::Closed
            } else {
                RoomAccess::NoBot
            },
        })
        .collect();
    Ok(Some(view::build(&ctx, &rooms, None)))
}
