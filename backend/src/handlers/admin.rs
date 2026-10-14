use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db,
    error::AppError,
    role::{self, Role},
    AppState,
};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/:id/roles", put(set_roles))
}

// GET /api/users — host-only. Lists every registered user with their linked
// Discord account (if any) folded in.
pub async fn list_users(
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
pub async fn set_roles(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<SetRolesBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let roles = role::normalize(body.roles);
    if user_id == user.id && !roles.contains(&Role::Host) {
        return Err(AppError::BadRequest("you can't remove your own host role"));
    }

    let updated = db::admin::set_roles(&state.db, user_id, &roles).await?;
    Ok(Json(updated))
}
