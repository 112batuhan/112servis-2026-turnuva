use axum::{
    extract::{Path, State},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{auth::AuthUser, db, error::AppError, role::Role, AppState};

// Routes under the `/api` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/:id/role", post(set_role))
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
pub struct SetRoleBody {
    role: Role,
}

// POST /api/users/{id}/role — host-only. Changes a user's role; the token_version
// bump inside set_role invalidates that user's existing JWTs (reconciled on their
// next /api/me).
pub async fn set_role(
    State(state): State<AppState>,
    user: AuthUser,
    Path(user_id): Path<Uuid>,
    Json(body): Json<SetRoleBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;

    let updated = db::admin::set_role(&state.db, user_id, body.role).await?;
    Ok(Json(updated))
}
