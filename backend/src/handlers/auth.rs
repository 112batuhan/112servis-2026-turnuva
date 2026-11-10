use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
    Router,
};
use axum_extra::extract::cookie::CookieJar;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    auth::{self, AuthUser, OAuthFlow},
    db,
    discord::{self, jobs::Job},
    discord_api,
    error::AppError,
    osu_api, rank_refresh, AppState,
};

// Routes under the `/auth` nest.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/osu", get(osu_login))
        .route("/osu/callback", get(osu_callback))
        .route("/discord/link", get(discord_link))
        .route("/discord/callback", get(discord_callback))
        .route("/logout", post(logout))
}

// Both OAuth providers redirect back with `?code=&state=`, or with `?error=` (and
// usually the state) when the user cancels on the consent screen.
#[derive(Debug, Deserialize)]
pub struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
}

// Why a callback didn't sign in or link, as the page is told it (snake_case in the
// query); the frontend shows each in Turkish (`LOGIN_ERROR_TEXT`, `DISCORD_LINK_ERROR_TEXT`
// in api/errors.ts).
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum CallbackError {
    // They cancelled on the provider's consent screen.
    Cancelled,
    // The flow ran out, or was started in another browser.
    Expired,
    // The provider or the database failed (the cause is in the log).
    Failed,
    // A Discord callback without a session.
    SignedOut,
    // The Discord account is linked to another osu! account.
    Taken,
    // The Discord link was started by another account than the one signed in now.
    WrongUser,
}

impl CallbackError {
    fn code(self) -> String {
        match serde_json::to_value(self) {
            Ok(serde_json::Value::String(code)) => code,
            _ => "failed".to_string(),
        }
    }

    // The provider sent the user back without a code: they cancelled, or it refused.
    fn of_provider(params: &CallbackParams) -> Self {
        match params.error.as_deref() {
            Some("access_denied") => CallbackError::Cancelled,
            _ => CallbackError::Failed,
        }
    }
}

// Where a failed callback sends the browser, with the code the page turns into a message.
fn osu_failed(state: &AppState, error: CallbackError) -> Redirect {
    Redirect::to(&format!(
        "{}/?login_error={}",
        state.config.frontend_url,
        error.code()
    ))
}

fn discord_failed(state: &AppState, error: CallbackError) -> Redirect {
    Redirect::to(&format!(
        "{}/discord?error={}",
        state.config.frontend_url,
        error.code()
    ))
}

// GET /auth/osu — starts the primary login flow.
async fn osu_login(State(state): State<AppState>, jar: CookieJar) -> impl IntoResponse {
    let (auth_url, csrf_token) = state
        .osu_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();
    let cookie = state.oauth_states.start(
        csrf_token.secret(),
        OAuthFlow::OsuLogin,
        None,
        state.config.secure_cookies(),
    );
    (jar.add(cookie), Redirect::to(auth_url.as_str()))
}

// GET /auth/osu/callback — exchanges the code, upserts the user, and starts a session.
async fn osu_callback(
    State(state): State<AppState>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> impl IntoResponse {
    let (jar, started) =
        state
            .oauth_states
            .finish(jar, params.state.as_deref(), OAuthFlow::OsuLogin);
    if started.is_none() {
        return (jar, osu_failed(&state, CallbackError::Expired));
    }
    let Some(code) = params.code.clone() else {
        return (jar, osu_failed(&state, CallbackError::of_provider(&params)));
    };
    match osu_sign_in(&state, code).await {
        Ok(user_id) => match auth::start_session(&state, user_id).await {
            Ok(cookie) => (jar.add(cookie), Redirect::to(&state.config.frontend_url)),
            Err(err) => {
                tracing::error!(error = ?err, "could not start a session");
                (jar, osu_failed(&state, CallbackError::Failed))
            }
        },
        Err(err) => {
            tracing::error!(error = ?err, "osu! login failed");
            (jar, osu_failed(&state, CallbackError::Failed))
        }
    }
}

// Exchanges an osu! login code and upserts the user, returning their id.
async fn osu_sign_in(state: &AppState, code: String) -> Result<Uuid, AppError> {
    let token = state
        .osu_client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&state.oauth_http)
        .await
        .map_err(|err| AppError::OsuTokenExchange(crate::error::chain(&err)))?;

    let profile =
        osu_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    let before = db::ranks::find_by_osu_id(&state.db, profile.id).await?;
    let user = db::user::upsert_osu_user(&state.db, &profile).await?;
    if let Some((user_id, old_rank)) = before {
        let change = db::ranks::RankChange {
            user_id,
            old_rank,
            new_rank: user.global_rank,
        };
        rank_refresh::notify_over_limit(state, &[change]).await;
    }
    Ok(user.id)
}

// GET /auth/discord/link — starts the optional Discord link flow. The AuthUser
// extractor rejects the request with 401 unless there's already an osu! session.
async fn discord_link(
    State(state): State<AppState>,
    user: AuthUser,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let mut request = state
        .discord_client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()));
    // Lets the bot add them to the tournament server as they link.
    if state.discord.enabled() {
        request = request.add_scope(Scope::new("guilds.join".to_string()));
    }
    let (auth_url, csrf_token) = request.url();
    let cookie = state.oauth_states.start(
        csrf_token.secret(),
        OAuthFlow::DiscordLink,
        Some(user.id),
        state.config.secure_cookies(),
    );
    Ok((jar.add(cookie), Redirect::to(auth_url.as_str())))
}

// Why linking failed, beyond what AppError says.
enum LinkError {
    // That Discord account is already linked to another osu! account.
    Taken,
    Failed(AppError),
}

impl<E: Into<AppError>> From<E> for LinkError {
    fn from(err: E) -> Self {
        LinkError::Failed(err.into())
    }
}

// GET /auth/discord/callback — links the Discord profile to the currently logged-in
// user, the one who started the flow.
async fn discord_callback(
    State(state): State<AppState>,
    user: Option<AuthUser>,
    Query(params): Query<CallbackParams>,
    jar: CookieJar,
) -> impl IntoResponse {
    let (jar, started) =
        state
            .oauth_states
            .finish(jar, params.state.as_deref(), OAuthFlow::DiscordLink);
    let Some(started_by) = started else {
        return (jar, discord_failed(&state, CallbackError::Expired));
    };
    // The link page needs a session, so this one is told on the landing page.
    let Some(user) = user else {
        return (jar, osu_failed(&state, CallbackError::SignedOut));
    };
    if started_by != Some(user.id) {
        return (jar, discord_failed(&state, CallbackError::WrongUser));
    }
    let Some(code) = params.code.clone() else {
        return (
            jar,
            discord_failed(&state, CallbackError::of_provider(&params)),
        );
    };
    match link_discord(&state, user, code).await {
        Ok(()) => (jar, Redirect::to(&state.config.frontend_url)),
        Err(LinkError::Taken) => (jar, discord_failed(&state, CallbackError::Taken)),
        Err(LinkError::Failed(err)) => {
            tracing::error!(error = ?err, "discord link failed");
            (jar, discord_failed(&state, CallbackError::Failed))
        }
    }
}

async fn link_discord(state: &AppState, user: AuthUser, code: String) -> Result<(), LinkError> {
    let token = state
        .discord_client
        .exchange_code(AuthorizationCode::new(code))
        .request_async(&state.oauth_http)
        .await
        .map_err(|err| AppError::DiscordTokenExchange(crate::error::chain(&err)))?;

    let profile =
        discord_api::user::fetch_current(&state.http_client, token.access_token().secret()).await?;

    // The session reads Discord status from the database, so the link applies to
    // the very next request with no cookie changes.
    let previous = db::discord::account_of(&state.db, user.id).await?;
    match db::user::link_discord(&state.db, user.id, &profile).await {
        Ok(_) => {}
        Err(sqlx::Error::Database(err))
            if err.constraint() == Some("discord_accounts_discord_id_key") =>
        {
            return Err(LinkError::Taken)
        }
        Err(err) => return Err(err.into()),
    }

    // Into the tournament server, then the roles. An account that was linked before
    // loses its roles.
    if let Some(bot) = state.discord.bot() {
        match discord::add_to_guild(bot, &profile.id, token.access_token().secret()).await {
            Ok(()) => {
                db::discord::set_in_guild(&state.db, &profile.id, true).await?;
            }
            Err(err) => {
                tracing::warn!(error = %err, "could not add a linked account to the server")
            }
        }
        let mut jobs = vec![Job::SyncUser { user_id: user.id }];
        if let Some((old, _)) = previous.filter(|(old, _)| *old != profile.id) {
            jobs.push(Job::SyncMember { discord_id: old });
        }
        jobs.extend(Job::sync_matches(
            db::discord::open_matches_of(&state.db, user.id).await?,
        ));
        state.discord.enqueue(&state.db, &jobs).await?;
    }
    Ok(())
}

// POST /auth/logout — deletes the session and clears its cookie.
async fn logout(
    State(state): State<AppState>,
    user: Option<AuthUser>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AppError> {
    let jar = auth::end_session(&state, jar).await?;
    if let Some(user) = user {
        state.live.user_changed(user.id);
    }
    Ok((jar, StatusCode::NO_CONTENT))
}
