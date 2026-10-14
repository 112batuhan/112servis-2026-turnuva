use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serenity::all::ChannelType;
use uuid::Uuid;

use crate::{
    auth::AuthUser,
    db::{self, discord::Settings, matches::MatchKind},
    discord::jobs::Job,
    error::AppError,
    handlers::matches::{load_match, require_runs},
    role::Role,
    AppState,
};

// Routes under the `/api` nest. Hosts set the bot up; spectators subscribe to bracket
// matches; referees ping players into their match.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/discord", get(get_discord))
        .route("/discord/settings", put(save_settings))
        .route("/discord/sync", post(sync))
        .route("/me/subscriptions", get(my_subscriptions))
        .route(
            "/bracket/:id/subscription",
            put(subscribe).delete(unsubscribe),
        )
        .route("/matches/:id/ping", post(ping))
}

#[derive(Debug, Serialize)]
pub struct GuildRole {
    id: String,
    name: String,
    color: u32,
}

#[derive(Debug, Serialize)]
pub struct GuildChannel {
    id: String,
    name: String,
    // The category it's in, for text channels.
    parent_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct GuildView {
    roles: Vec<GuildRole>,
    text_channels: Vec<GuildChannel>,
    categories: Vec<GuildChannel>,
}

#[derive(Debug, Serialize)]
pub struct DiscordView {
    // Whether DISCORD_BOT_TOKEN and DISCORD_GUILD_ID are set.
    enabled: bool,
    connected: bool,
    // The server's roles and channels to pick from; null when the bot is off or
    // Discord couldn't be reached.
    guild: Option<GuildView>,
    settings: Settings,
}

// GET /api/discord — host-only. The bot's state, its settings and what can be picked.
pub async fn get_discord(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let settings = db::discord::settings(&state.db).await?;
    let Some(bot) = state.discord.bot() else {
        return Ok(Json(DiscordView {
            enabled: false,
            connected: false,
            guild: None,
            settings,
        }));
    };
    let guild = async {
        let roles = bot.guild_id.roles(&bot.http).await?;
        let channels = bot.guild_id.channels(&bot.http).await?;
        Ok::<_, serenity::Error>((roles, channels))
    }
    .await;
    let guild = match guild {
        Ok((roles, channels)) => {
            let mut roles: Vec<_> = roles
                .into_values()
                .filter(|r| r.id.get() != bot.guild_id.get() && !r.managed)
                .collect();
            roles.sort_by_key(|r| std::cmp::Reverse(r.position));
            let mut channels: Vec<_> = channels.into_values().collect();
            channels.sort_by_key(|c| c.position);
            let view = |c: &serenity::all::GuildChannel| GuildChannel {
                id: c.id.to_string(),
                name: c.name.clone(),
                parent_id: c.parent_id.map(|p| p.to_string()),
            };
            Some(GuildView {
                roles: roles
                    .iter()
                    .map(|r| GuildRole {
                        id: r.id.to_string(),
                        name: r.name.clone(),
                        color: r.colour.0,
                    })
                    .collect(),
                text_channels: channels
                    .iter()
                    .filter(|c| matches!(c.kind, ChannelType::Text | ChannelType::News))
                    .map(view)
                    .collect(),
                categories: channels
                    .iter()
                    .filter(|c| c.kind == ChannelType::Category)
                    .map(view)
                    .collect(),
            })
        }
        Err(err) => {
            tracing::warn!(error = %err, "could not read the discord server");
            None
        }
    };
    Ok(Json(DiscordView {
        enabled: true,
        connected: bot.connected(),
        guild,
        settings,
    }))
}

// PUT /api/discord/settings — host-only. Saves what the bot uses, then brings the
// server in line with it (roles handed out, channels moved to new categories).
pub async fn save_settings(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<Settings>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::discord::save_settings(&state.db, &body).await?;
    state.discord.enqueue(&state.db, &[Job::SyncAll]).await?;
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/discord/sync — host-only. Rechecks who's in the server, everyone's roles
// and every match channel.
pub async fn sync(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !state.discord.enabled() {
        return Err(AppError::Conflict("the Discord bot isn't set up"));
    }
    state.discord.enqueue(&state.db, &[Job::SyncAll]).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Serialize)]
pub struct Subscriptions {
    match_ids: Vec<Uuid>,
    // Subscribing needs the bot on and the caller's Discord linked.
    enabled: bool,
    discord_linked: bool,
}

// GET /api/me/subscriptions — the bracket matches the caller wants to be pinged for.
pub async fn my_subscriptions(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    Ok(Json(Subscriptions {
        match_ids: db::discord::subscriptions(&state.db, user.id).await?,
        enabled: state.discord.enabled(),
        discord_linked: user.discord_verified,
    }))
}

// PUT /api/bracket/:id/subscription — pings the caller on Discord when the match is
// about to start. Needs a linked Discord account.
pub async fn subscribe(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_discord_verified()?;
    let m = load_match(&state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(AppError::NotFound);
    }
    if m.concluded() {
        return Err(AppError::Conflict("this match is already over"));
    }
    db::discord::subscribe(&state.db, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket/:id/subscription
pub async fn unsubscribe(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    db::discord::unsubscribe(&state.db, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct PingBody {
    user_id: Uuid,
}

// POST /api/matches/:id/ping — the match's referee (or a host) pings one of its
// players in the match's Discord channel, at most every 30 seconds per player.
pub async fn ping(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PingBody>,
) -> Result<impl IntoResponse, AppError> {
    let m = load_match(&state, id).await?;
    require_runs(&user, &m)?;
    let Some(bot) = state.discord.bot() else {
        return Err(AppError::Conflict("the Discord bot isn't set up"));
    };
    let players = db::discord::match_players(&state.db, id).await?;
    let player = players
        .iter()
        .find(|p| p.user_id == body.user_id)
        .ok_or(AppError::BadRequest("that player isn't in this match"))?;
    if player.discord_id.is_none() || player.in_guild == Some(false) {
        return Err(AppError::Conflict(
            "that player isn't in the Discord server",
        ));
    }
    if db::discord::channel_of(&state.db, id).await?.is_none() {
        return Err(AppError::Conflict("this match has no Discord channel yet"));
    }
    if !bot.take_ping(body.user_id) {
        return Err(AppError::Conflict(
            "that player was just pinged, wait a little",
        ));
    }
    state
        .discord
        .enqueue(
            &state.db,
            &[Job::Ping {
                match_id: id,
                user_id: body.user_id,
                by: user.id,
            }],
        )
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
