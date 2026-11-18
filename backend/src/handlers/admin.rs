use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db,
    discord::jobs::Job,
    error::{AppError, AuthError, GeneralError},
    role::{self, Role},
    AppState,
};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/roles", put(set_roles))
        .route("/users/{id}/ban", put(ban_user).delete(unban_user))
}

// GET /api/users — host-only. Lists every registered user with their linked
// Discord account (if any) folded in.
async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;

    let users = db::admin::list_users(&state.db).await?;
    Ok(Json(users))
}

#[derive(Debug, Deserialize)]
pub struct SetRolesBody {
    roles: Vec<Role>,
}

// PUT /api/users/{id}/roles — host-only. Replaces a user's roles (any set; empty makes
// them a regular player), effective on their next request since sessions read the
// roles from the database. A host can't take Host off themselves, so the site can't
// be left without one by a misclick.
async fn set_roles(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<SetRolesBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let roles = role::normalize(body.roles);
    if user_id == user.id && !roles.contains(&Role::Host) {
        return Err(AuthError::OwnHostRole.into());
    }

    let updated = db::admin::set_roles(&state.db, user_id, &roles).await?;
    // Who the bots let stay in a room, and make its referees, goes by the roles.
    state.bancho.resync();
    // Their open sockets (staff events) check again.
    state.live.user_changed(user_id);
    Ok(Json(updated))
}

const BAN_REASON_MAX: usize = 300;

#[derive(Debug, Deserialize)]
pub struct BanBody {
    #[serde(default)]
    reason: String,
}

// PUT /api/users/{id}/ban — host-only. Bans a user from taking part (see
// AuthUser::require_not_banned), or changes the reason of their ban. Their pending invites
// go; a team they're already in stays until a host disbands it. A new ban notifies them,
// their teammate and the other hosts, on the site and on Discord.
async fn ban_user(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<BanBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if user_id == user.id {
        return Err(AuthError::OwnBan.into());
    }
    let reason = body.reason.trim();
    if reason.chars().count() > BAN_REASON_MAX {
        return Err(AuthError::BanReasonLength.into());
    }
    let mut tx = state.db.begin().await?;
    let write = db::ban::ban(&mut tx, user_id, reason, user.id)
        .await?
        .ok_or(GeneralError::NotFound)?;
    // A new ban tells the player, their teammate and the other hosts; changing its reason
    // tells no one.
    let notified = if write.new {
        let notified = db::notifications::notify_banned(&mut tx, user_id, user.id, reason).await?;
        state
            .discord
            .enqueue(&mut *tx, &[Job::UserBanned { user_id }])
            .await?;
        notified
    } else {
        Vec::new()
    };
    tx.commit().await?;
    state.live.notifications_changed(notified);
    Ok(Json(write.ban))
}

// DELETE /api/users/{id}/ban — host-only. Lifts a ban.
async fn unban_user(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !db::ban::unban(&state.db, user_id).await? {
        return Err(GeneralError::NotFound.into());
    }
    Ok(StatusCode::NO_CONTENT)
}
