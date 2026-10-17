use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{patch, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    bancho::{BotLimits, BotStatus, RoomCommand, LIMITS},
    db::{self, bancho::BotRow},
    error::AppError,
    handlers::matches::{load_match, require_runs},
    role::Role,
    AppState,
};

// Routes under the `/api` nest. Hosts manage the bot accounts on /admin/bancho; the
// referee page asks the room's bot to make, set up and run the room.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/bancho/bots",
            axum::routing::get(list_bots).post(create_bot),
        )
        .route("/bancho/bots/:id", patch(update_bot).delete(delete_bot))
        .route("/matches/:id/room/create", post(create_room))
        .route("/matches/:id/room/command", post(command))
        .route("/matches/:id/room/save-score", post(save_score))
}

#[derive(Debug, Serialize)]
pub struct BotView {
    #[serde(flatten)]
    account: BotRow,
    // None while the manager hasn't heard of it yet.
    status: Option<BotStatus>,
}

#[derive(Debug, Serialize)]
pub struct BotsView {
    bots: Vec<BotView>,
    limits: BotLimits,
}

// GET /api/bancho/bots — host-only. Every bot account, how it's doing and its rooms.
pub async fn list_bots(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let accounts = db::bancho::list_bots(&state.db).await?;
    let mut status = state.bancho.status().await;
    Ok(Json(BotsView {
        bots: accounts
            .into_iter()
            .map(|account| {
                let i = status.iter().position(|s| s.id == account.id);
                BotView {
                    status: i.map(|i| status.swap_remove(i)),
                    account,
                }
            })
            .collect(),
        limits: LIMITS,
    }))
}

// osu! usernames: 2-15 letters, digits, spaces and -_[].
fn valid_username(name: &str) -> bool {
    (2..=15).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || " -_[]".contains(c))
}

fn valid_password(password: &str) -> bool {
    !password.is_empty()
        && password.len() <= 64
        && !password
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
}

#[derive(Debug, Deserialize)]
pub struct CreateBotBody {
    username: String,
    irc_password: String,
    #[serde(default)]
    active: bool,
}

#[derive(Debug, Serialize)]
pub struct Created {
    id: Uuid,
}

// POST /api/bancho/bots — host-only. Adds an osu! account the site runs rooms with.
pub async fn create_bot(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateBotBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let username = body.username.trim();
    if !valid_username(username) {
        return Err(AppError::BadRequest("that isn't an osu! username"));
    }
    if !valid_password(&body.irc_password) {
        return Err(AppError::BadRequest("that isn't an IRC password"));
    }
    let id = match db::bancho::create_bot(&state.db, username, &body.irc_password).await {
        Ok(id) => id,
        Err(sqlx::Error::Database(e)) if e.constraint() == Some("bancho_bots_username_key") => {
            return Err(AppError::Conflict("that account is already a bot"))
        }
        Err(err) => return Err(err.into()),
    };
    if body.active {
        db::bancho::set_active(&state.db, id, true).await?;
    }
    state.bancho.bots_changed();
    Ok((StatusCode::CREATED, Json(Created { id })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateBotBody {
    irc_password: Option<String>,
    active: Option<bool>,
}

// PATCH /api/bancho/bots/:id — host-only. A new IRC password, or switches it on or off.
// Switching a bot off leaves its rooms on their matches, without a bot until it's back.
pub async fn update_bot(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<UpdateBotBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if let Some(password) = &body.irc_password {
        if !valid_password(password) {
            return Err(AppError::BadRequest("that isn't an IRC password"));
        }
        if !db::bancho::set_password(&state.db, id, password).await? {
            return Err(AppError::NotFound);
        }
    }
    if let Some(active) = body.active {
        if !db::bancho::set_active(&state.db, id, active).await? {
            return Err(AppError::NotFound);
        }
    }
    state.bancho.bots_changed();
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bancho/bots/:id — host-only. Its rooms stay on their matches without a bot.
pub async fn delete_bot(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::bancho::delete_bot(&state.db, id).await? {
        return Err(AppError::NotFound);
    }
    state.bancho.bots_changed();
    state.bancho.resync();
    Ok(StatusCode::NO_CONTENT)
}

// A room is run by any referee (or a host) until the match is concluded.
async fn check_runs(state: &AppState, user: &AuthUser, id: Uuid) -> Result<(), AppError> {
    let m = load_match(state, id).await?;
    require_runs(user)?;
    if m.concluded() {
        return Err(AppError::Conflict("this match is concluded"));
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct CreatedRoom {
    mp_id: i64,
}

// POST /api/matches/:id/room/create — the least busy bot with room for it makes the
// match's room (or a new one, to replace a broken room), sets it up and invites the
// players. Refused when every bot already has as many rooms as Bancho allows.
pub async fn create_room(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_runs(&state, &user, id).await?;
    let mp_id = state.bancho.create_room(id).await?;
    Ok(Json(CreatedRoom { mp_id }))
}

// POST /api/matches/:id/room/command — the room's bot sends a command to the match's
// latest room. Saying something through it (`say`) is for hosts, who referee from the
// site: their accounts are the bots.
pub async fn command(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(mut command): Json<RoomCommand>,
) -> Result<impl IntoResponse, AppError> {
    check_runs(&state, &user, id).await?;
    if let RoomCommand::Say { text, by } = &mut command {
        if !user.roles.has(Role::Host) {
            return Err(AppError::Forbidden);
        }
        if text.trim().is_empty() {
            return Err(AppError::BadRequest("write something first"));
        }
        let profile = db::user::find_user(&state.db, user.id)
            .await?
            .ok_or(AppError::Unauthenticated)?;
        *by = profile.osu.username;
    }
    state.bancho.command(id, command).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/matches/:id/room/save-score — saves the score as the bots count it, now
// (they save it by themselves as it changes, but not over a score entered by hand).
pub async fn save_score(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    check_runs(&state, &user, id).await?;
    state.bancho.save_score(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
