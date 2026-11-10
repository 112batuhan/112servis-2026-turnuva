//! The tournament server's Discord bot.
//!
//! It runs only when DISCORD_BOT_TOKEN and DISCORD_GUILD_ID are set; everything it
//! points at in the server is picked by hosts on /admin/discord (db::discord::Settings).
//!
//! The site never calls Discord while handling a request. Handlers queue jobs
//! (`Discord::enqueue`) in the same transaction as the change they're about, and one
//! worker runs them in order (jobs.rs), retrying failures. Jobs that announce something
//! render it when they run and queue a `Post` per destination, so a failed send is
//! retried on its own without repeating the others. Alongside it run the gateway
//! connection (events.rs: members joining and leaving, slash commands and buttons) and
//! the scheduler (scheduler.rs: reminders and announcements before matches start).
mod announcement;
mod channels;
mod commands;
mod events;
pub mod jobs;
mod members;
mod messages;
mod results;
mod scheduler;
pub mod snowflake;

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use serenity::{
    all::{GatewayIntents, GuildId, Http, UserId},
    http::{HttpBuilder, Ratelimiter},
    Client,
};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{config::DiscordBotConfig, db, AppState};
use jobs::Job;
pub use members::add_to_guild;

// How often a referee can ping the same player.
const PING_COOLDOWN: Duration = Duration::from_secs(30);

// Serenity's HTTP client has no timeouts of its own, so a request Discord never answers
// would hold up the job worker for good.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
// Rate-limit waits at least this long are logged: serenity sleeps through them silently,
// and the job worker waits with it.
const LOG_RATELIMIT_OVER: Duration = Duration::from_secs(2);

// The bot's HTTP client: serenity's, with timeouts and a warning for long rate-limit
// waits.
fn http_client(token: &str) -> Http {
    let client = reqwest::Client::builder()
        .use_rustls_tls()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .expect("could not build the discord http client");
    // The ratelimiter sends the token as is, so it needs the prefix serenity adds.
    let token = token.trim();
    let auth = if token.starts_with("Bot ") {
        token.to_string()
    } else {
        format!("Bot {token}")
    };
    let mut ratelimiter = Ratelimiter::new(client.clone(), auth);
    ratelimiter.set_ratelimit_callback(Box::new(|info| {
        if info.timeout >= LOG_RATELIMIT_OVER {
            tracing::warn!(
                method = ?info.method,
                path = %info.path,
                wait_secs = info.timeout.as_secs_f64(),
                global = info.global,
                "discord rate limit, waiting"
            );
        }
    }));
    HttpBuilder::new(token)
        .client(client)
        .ratelimiter(ratelimiter)
        .build()
}

pub struct Bot {
    pub http: Arc<Http>,
    pub guild_id: GuildId,
    token: String,
    // The bot's own user, read once; its channel overwrites name it.
    user_id: tokio::sync::OnceCell<UserId>,
    // When each player was last pinged from the referee page.
    pings: Mutex<HashMap<Uuid, Instant>>,
    // Whether the gateway is connected, for the admin page.
    connected: AtomicBool,
    // Whether hosts have it switched on (`discord_settings.bot_active`). While it's off
    // the gateway is closed, the worker and the scheduler wait and nothing is queued.
    active: tokio::sync::watch::Sender<bool>,
}

impl Bot {
    pub async fn user_id(&self) -> BotResult<UserId> {
        self.user_id
            .get_or_try_init(|| async { Ok::<_, BotError>(self.http.get_current_user().await?.id) })
            .await
            .copied()
    }

    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    pub fn active(&self) -> bool {
        *self.active.borrow()
    }

    pub fn set_active(&self, active: bool) {
        self.active.send_replace(active);
    }

    // Returns once the bot is switched on (`true`) or off (`false`).
    async fn until_active(&self, active: bool) {
        let mut rx = self.active.subscribe();
        // The sender lives as long as the bot, so this can't fail.
        let _ = rx.wait_for(|a| *a == active).await;
    }

    // Allows a ping unless the player was pinged within the cooldown.
    pub fn take_ping(&self, user_id: Uuid) -> bool {
        let mut pings = self.pings.lock().unwrap();
        let now = Instant::now();
        pings.retain(|_, at| now.duration_since(*at) < PING_COOLDOWN);
        if pings.contains_key(&user_id) {
            return false;
        }
        pings.insert(user_id, now);
        true
    }
}

// The bot, when it's configured. Cloned into AppState.
#[derive(Clone, Default)]
pub struct Discord(Option<Arc<Bot>>);

impl Discord {
    pub fn new(config: &DiscordBotConfig) -> Self {
        let Some((token, guild_id)) = config.bot() else {
            return Self(None);
        };
        Self(Some(Arc::new(Bot {
            http: Arc::new(http_client(token)),
            guild_id: GuildId::new(guild_id),
            token: token.to_string(),
            user_id: tokio::sync::OnceCell::new(),
            pings: Mutex::new(HashMap::new()),
            connected: AtomicBool::new(false),
            // Read from the database in `spawn`, before any request is served.
            active: tokio::sync::watch::Sender::new(false),
        })))
    }

    // The bot while it runs: configured and switched on.
    pub fn bot(&self) -> Option<&Arc<Bot>> {
        self.0.as_ref().filter(|bot| bot.active())
    }

    // The bot when it's configured, switched on or not.
    pub fn configured(&self) -> Option<&Arc<Bot>> {
        self.0.as_ref()
    }

    pub fn enabled(&self) -> bool {
        self.bot().is_some()
    }

    // Queues jobs for the worker, in order. Nothing is queued while the bot is off
    // (unconfigured or switched off), so turning it on later doesn't replay the past; a
    // full sync catches it up instead.
    pub async fn enqueue(&self, ex: impl PgExecutor<'_>, jobs: &[Job]) -> sqlx::Result<()> {
        if !self.enabled() || jobs.is_empty() {
            return Ok(());
        }
        db::discord::push_jobs(ex, jobs).await
    }
}

// What can go wrong running a job.
#[derive(Debug, thiserror::Error)]
pub enum BotError {
    #[error("discord: {0}")]
    // Boxed: serenity's error is large, and every bot function returns this.
    Discord(Box<serenity::Error>),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    App(#[from] crate::error::AppError),
    // Something picked on /admin/discord doesn't exist in the server.
    #[error("{0}")]
    Setup(String),
    // The job took longer than jobs::JOB_TIMEOUT, most likely waiting out a Discord rate
    // limit (serenity sleeps through those); it's retried later.
    #[error("timed out")]
    Timeout,
}

impl From<serenity::Error> for BotError {
    fn from(err: serenity::Error) -> Self {
        BotError::Discord(Box::new(err))
    }
}

impl BotError {
    // Whether retrying can't help: Discord refused the request itself (missing
    // permissions, unknown channel or user, closed DMs) rather than failing to answer.
    pub fn permanent(&self) -> bool {
        match self {
            BotError::Discord(err) => match err.as_ref() {
                serenity::Error::Http(err) => err
                    .status_code()
                    .is_some_and(|s| s.is_client_error() && s.as_u16() != 429),
                serenity::Error::Model(_) => true,
                _ => false,
            },
            BotError::Setup(_) => true,
            _ => false,
        }
    }

    // Discord answered that the thing doesn't exist (404).
    pub fn not_found(&self) -> bool {
        matches!(self, BotError::Discord(err)
            if matches!(err.as_ref(), serenity::Error::Http(err)
                if err.status_code().is_some_and(|s| s.as_u16() == 404)))
    }
}

pub type BotResult<T = ()> = Result<T, BotError>;

// Reads whether hosts left the bot switched on, then starts the gateway connection, the
// job worker and the scheduler. Each of them waits while the bot is switched off.
pub async fn spawn(state: &AppState) {
    let Some(bot) = state.discord.configured().cloned() else {
        tracing::info!("discord bot is off (DISCORD_BOT_TOKEN / DISCORD_GUILD_ID unset)");
        return;
    };
    let active = db::discord::bot_active(&state.db)
        .await
        .unwrap_or_else(|err| {
            tracing::error!(error = %err, "could not read whether the discord bot is on");
            true
        });
    bot.set_active(active);
    if !active {
        tracing::info!("discord bot is switched off on /admin/discord");
    }
    {
        let (state, bot) = (state.clone(), bot.clone());
        crate::supervise::spawn("discord job worker", move || {
            jobs::run_worker(state.clone(), bot.clone())
        });
    }
    {
        let state = state.clone();
        crate::supervise::spawn("discord scheduler", move || scheduler::run(state.clone()));
    }
    let state = state.clone();
    crate::supervise::spawn("discord gateway", move || {
        gateway(state.clone(), bot.clone())
    });
}

// Keeps the gateway connected while the bot is switched on.
async fn gateway(state: AppState, bot: Arc<Bot>) {
    {
        // GUILD_MEMBERS is privileged: it has to be enabled on the bot's page.
        let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;
        loop {
            bot.until_active(true).await;
            let handler = events::Handler {
                state: state.clone(),
                bot: bot.clone(),
            };
            let client = Client::builder(&bot.token, intents)
                .event_handler(handler)
                .await;
            match client {
                Ok(mut client) => {
                    // Switching the bot off closes the connection, which ends `start`.
                    let shards = client.shard_manager.clone();
                    let closer = tokio::spawn({
                        let bot = bot.clone();
                        async move {
                            bot.until_active(false).await;
                            shards.shutdown_all().await;
                        }
                    });
                    if let Err(err) = client.start().await {
                        tracing::error!(error = %err, "discord gateway stopped");
                    }
                    closer.abort();
                    bot.connected.store(false, Ordering::Relaxed);
                    if !bot.active() {
                        tracing::info!("discord bot switched off");
                        continue;
                    }
                }
                Err(err) => tracing::error!(error = %err, "could not start the discord client"),
            }
            // Retry in a while, unless it's switched off meanwhile.
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(30)) => {}
                _ = bot.until_active(false) => {}
            }
        }
    }
}

// A match's title as its Discord messages show it ("W3: Team A vs Team B"), for jobs
// that outlive the match.
pub fn announcement_title(info: &crate::db::discord::MatchInfo) -> String {
    messages::title(info)
}
