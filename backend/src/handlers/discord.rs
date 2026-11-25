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

use super::require_runs;
use crate::{
    auth::AuthUser,
    db::{self, discord::Settings, matches::MatchKind},
    discord::jobs::Job,
    error::{AppError, DiscordError, GeneralError, MatchError},
    role::Role,
    services::matches::load_match,
    AppState,
};

// Routes under the `/api` nest. Hosts set the bot up; spectators subscribe to bracket
// matches; referees ping players into their match.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/discord", get(get_discord))
        .route("/discord/settings", put(save_settings))
        .route("/discord/sync", post(sync))
        .route("/discord/active", put(set_active))
        .route("/discord/channels/paused", put(set_channels_paused))
        .route("/discord/channels/delete", post(delete_channels))
        .route("/me/subscriptions", get(my_subscriptions))
        .route(
            "/bracket/{id}/subscription",
            put(subscribe).delete(unsubscribe),
        )
        .route("/matches/{id}/ping", post(ping))
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
    // Whether hosts have it switched on.
    active: bool,
    connected: bool,
    // Whether hosts have stopped new match channels from opening.
    channels_paused: bool,
    // How many matches have a channel.
    channel_count: i64,
    // The server's roles and channels to pick from; null when the bot is off or
    // Discord couldn't be reached.
    guild: Option<GuildView>,
    settings: Settings,
}

// GET /api/discord — host-only. The bot's state, its settings and what can be picked.
async fn get_discord(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let settings = db::discord::settings(&state.db).await?;
    let channels_paused = db::discord::channels_paused(&state.db).await?;
    let channel_count = db::discord::channel_count(&state.db).await?;
    // The server can be read while the bot is switched off, so it can be set up first.
    let Some(bot) = state.discord.configured() else {
        return Ok(Json(DiscordView {
            enabled: false,
            active: false,
            connected: false,
            channels_paused,
            channel_count,
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
        active: bot.active(),
        connected: bot.connected(),
        channels_paused,
        channel_count,
        guild,
        settings,
    }))
}

// PUT /api/discord/settings — host-only. Saves what the bot uses, then brings the
// server in line with it (roles handed out, channels moved to new categories).
async fn save_settings(
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
async fn sync(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !state.discord.enabled() {
        return Err(DiscordError::BotOff.into());
    }
    state.discord.enqueue(&state.db, &[Job::SyncAll]).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct ActiveBody {
    active: bool,
}

// PUT /api/discord/active — host-only. Switches the bot off (the gateway closes, jobs
// still waiting are dropped and nothing new is queued) or back on (it reconnects and
// runs a full sync, which catches up on what happened meanwhile).
async fn set_active(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<ActiveBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    let Some(bot) = state.discord.configured() else {
        return Err(DiscordError::BotMissing.into());
    };
    // Off in memory first, so nothing is queued after the queue is cleared.
    let was = bot.active();
    if !body.active {
        bot.set_active(false);
    }
    if let Err(err) = db::discord::set_bot_active(&state.db, body.active).await {
        bot.set_active(was);
        return Err(err.into());
    }
    bot.set_active(body.active);
    tracing::info!(by = %user.id, active = body.active, "discord bot switched");
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
pub struct PausedBody {
    paused: bool,
}

// PUT /api/discord/channels/paused — host-only. Stops new match channels from opening
// and the bot from sending any message (the ones there are still kept up to date and
// archived), or lets them open again, in which case every match that needs one gets it.
async fn set_channels_paused(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<PausedBody>,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    db::discord::set_channels_paused(&state.db, body.paused).await?;
    if !body.paused {
        let jobs = Job::sync_matches(db::discord::syncable_matches(&state.db).await?);
        state.discord.enqueue(&state.db, &jobs).await?;
    }
    tracing::info!(by = %user.id, paused = body.paused, "discord match channels switched");
    Ok(StatusCode::NO_CONTENT)
}

// POST /api/discord/channels/delete — host-only. Deletes every match channel, archived
// ones included, in the background. Unless channels are paused, the matches that need
// one get a new one afterwards.
async fn delete_channels(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<impl IntoResponse, AppError> {
    user.require_role(Role::Host)?;
    if !state.discord.enabled() {
        return Err(DiscordError::BotOff.into());
    }
    state
        .discord
        .enqueue(&state.db, &[Job::DeleteMatchChannels])
        .await?;
    tracing::info!(by = %user.id, "deleting every discord match channel");
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
async fn my_subscriptions(
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
async fn subscribe(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, AppError> {
    user.require_discord_verified()?;
    let m = load_match(&state, id).await?;
    if m.kind != MatchKind::Bracket {
        return Err(GeneralError::NotFound.into());
    }
    if m.concluded() {
        return Err(MatchError::Concluded.into());
    }
    db::discord::subscribe(&state.db, id, user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// DELETE /api/bracket/:id/subscription
async fn unsubscribe(
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

// POST /api/matches/:id/ping — a referee (or a host) pings one of its
// players in the match's Discord channel, at most every 30 seconds per player.
async fn ping(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<Uuid>,
    Json(body): Json<PingBody>,
) -> Result<impl IntoResponse, AppError> {
    require_runs(&user)?;
    // The match has to exist.
    load_match(&state, id).await?;
    let Some(bot) = state.discord.bot() else {
        return Err(DiscordError::BotOff.into());
    };
    let players = db::discord::match_players(&state.db, id).await?;
    let player = players
        .iter()
        .find(|p| p.user_id == body.user_id)
        .ok_or(MatchError::PlayerNotInMatch)?;
    if player.discord_id.is_none() || player.in_guild == Some(false) {
        return Err(DiscordError::PlayerNotInServer.into());
    }
    if db::discord::channel_of(&state.db, id).await?.is_none() {
        return Err(DiscordError::NoChannel.into());
    }
    if db::discord::channels_paused(&state.db).await? {
        return Err(DiscordError::ChannelsPaused.into());
    }
    if !bot.take_ping(body.user_id) {
        return Err(DiscordError::PingCooldown.into());
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
