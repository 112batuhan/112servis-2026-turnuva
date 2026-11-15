//! HTTP request handlers, grouped by feature area.
//!
//! Handlers return `Result<impl IntoResponse, crate::error::AppError>` and pull
//! shared services (db pool, http client, oauth clients, config) off `AppState`. They
//! check who's asking and what was sent; what's done to the data by more than the
//! routes (the bots, Discord) is in `crate::services`, the response shapes several of
//! them share in `crate::views`, and the checks they share are here.
pub mod admin;
pub mod arg;
pub mod auth;
pub mod bancho;
pub mod bracket;
pub mod dev;
pub mod discord;
pub mod irc;
pub mod live;
pub mod mappool;
pub mod matches;
pub mod notifications;
pub mod pickem;
pub mod referee;
pub mod reschedule;
pub mod rules;
pub mod seeding;
pub mod stages;
pub mod stats;
pub mod team;
pub mod user;

use axum::http::{header, HeaderMap};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::matches::MatchState,
    error::{AppError, AuthError, MatchError},
    role::Role,
    AppState,
};

// CORS doesn't cover WebSockets, so the socket routes check that the page asking is ours.
fn check_origin(state: &AppState, headers: &HeaderMap) -> Result<(), AppError> {
    if let Some(origin) = headers.get(header::ORIGIN) {
        let ours = state.config.frontend_url.trim_end_matches('/');
        if origin.as_bytes() != ours.as_bytes() {
            return Err(AuthError::Forbidden.into());
        }
    }
    Ok(())
}

fn is_host(user: &AuthUser) -> bool {
    user.roles.has(Role::Host)
}

// Any referee (or host) can run any match, so one can step in when its own referee
// can't; the assigned referee is the one Discord pings and asks about the match.
fn require_runs(user: &AuthUser) -> Result<(), AppError> {
    user.require_role(Role::Referee)
}

// Once a match is concluded only hosts can change it (notes aside).
fn require_unconcluded(user: &AuthUser, m: &MatchState) -> Result<(), AppError> {
    if m.concluded() && !is_host(user) {
        return Err(MatchError::Concluded.into());
    }
    Ok(())
}

// A new ordering for a set of items, most-first, given by their ids.
#[derive(Debug, Deserialize)]
struct ReorderBody {
    ids: Vec<Uuid>,
}

#[cfg(test)]
mod tests {
    use axum::Router;

    use crate::AppState;

    // Several handlers add methods to the same path (GET /stages here, POST /stages
    // there); merging panics if two of them claim the same method on one.
    #[test]
    fn route_sets_merge_without_collisions() {
        let _: Router<AppState> = Router::new()
            .merge(super::user::routes())
            .merge(super::admin::routes())
            .merge(super::arg::routes())
            .merge(super::mappool::routes())
            .merge(super::stages::routes())
            .merge(super::matches::routes())
            .merge(super::bracket::routes())
            .merge(super::referee::routes())
            .merge(super::reschedule::routes())
            .merge(super::rules::routes())
            .merge(super::seeding::routes())
            .merge(super::stats::routes())
            .merge(super::notifications::routes())
            .merge(super::pickem::routes())
            .merge(super::irc::routes())
            .merge(super::bancho::routes())
            .merge(super::live::routes())
            .merge(super::discord::routes())
            .merge(super::team::routes())
            .merge(super::dev::routes());
    }
}
