use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_extra::extract::cookie::CookieJar;
use uuid::Uuid;

use crate::{
    auth, db,
    error::{AppError, AuthError, GeneralError, TeamError},
    AppState,
};

// Testing helpers, mounted only when DEV_LOGIN is on. They let anyone sign in as
// any account without osu!, so turn it off once the site goes public.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dev/accounts", get(list_accounts))
        .route("/dev/login/{id}", post(login_as))
        .route("/dev/seed", post(seed).delete(clear))
        .route("/dev/seed/live", post(seed_live))
        .route("/dev/seed/qualifiers", post(seed_qualifiers))
        .route("/dev/seed/teams", post(seed_teams))
}

async fn list_accounts(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    Ok(Json(db::dev::list_accounts(&state.db).await?))
}

// POST /api/dev/login/:id — replaces the current session with one for `id`.
async fn login_as(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    if !db::dev::user_exists(&state.db, id).await? {
        return Err(GeneralError::NotFound.into());
    }
    let jar = auth::end_session(&state, jar).await?;
    let jar = jar.add(auth::start_session(&state, id).await?);
    Ok((jar, StatusCode::NO_CONTENT))
}

async fn seed(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    db::dev::seed(&state.db).await?;
    crate::services::bracket::resync_potentials(&state).await?;
    // The first lobby is seeded with only its chat.
    crate::bancho::backfill_lobbies(&state).await?;
    // The bots read the seeded rooms; the open pages reload.
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/dev/seed/qualifiers — reseeds as the tournament just after its qualifier:
// every lobby played, no bracket yet (seed/qualifiers.sql), for the seeding page.
async fn seed_qualifiers(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    db::dev::seed_qualifiers(&state.db).await?;
    crate::services::bracket::resync_potentials(&state).await?;
    crate::bancho::backfill_lobbies(&state).await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/dev/seed/teams — reseeds with only a few teams and the dev staff: no
// lobby, score or bracket (seed/teams.sql), to see the lists without qualifier places.
async fn seed_teams(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    db::dev::seed_teams(&state.db).await?;
    crate::services::bracket::resync_potentials(&state).await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/dev/seed/live — reseeds, then puts the signed-in real account in a team
// with a bracket match starting now (seed/live.sql), to test a real osu! room.
async fn seed_live(
    State(state): State<AppState>,
    user: auth::AuthUser,
) -> Result<impl IntoResponse, AppError> {
    match db::dev::seed_live(&state.db, user.id).await? {
        db::dev::LiveSeed::Done => {
            crate::services::bracket::resync_potentials(&state).await?;
            crate::bancho::backfill_lobbies(&state).await?;
            state.live.bracket_changed();
            Ok(StatusCode::NO_CONTENT)
        }
        db::dev::LiveSeed::Dummy => Err(AuthError::DevLiveNeedsRealAccount.into()),
        db::dev::LiveSeed::InTeam => Err(TeamError::AlreadyInTeam.into()),
    }
}

async fn clear(State(state): State<AppState>) -> Result<impl IntoResponse, AppError> {
    db::dev::clear(&state.db).await?;
    crate::services::bracket::resync_potentials(&state).await?;
    state.live.bracket_changed();
    Ok(StatusCode::NO_CONTENT)
}
