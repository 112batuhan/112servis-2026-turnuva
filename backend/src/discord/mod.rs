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
    Client,
};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::{config::DiscordBotConfig, db, AppState};
use jobs::Job;
pub use members::add_to_guild;

// How often a referee can ping the same player.
const PING_COOLDOWN: Duration = Duration::from_secs(30);

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
}

impl Bot {
    pub async fn user_id(&self) -> serenity::Result<UserId> {
        self.user_id
            .get_or_try_init(|| async { Ok(self.http.get_current_user().await?.id) })
            .await
            .copied()
    }

    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
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
            http: Arc::new(Http::new(token)),
            guild_id: GuildId::new(guild_id),
            token: token.to_string(),
            user_id: tokio::sync::OnceCell::new(),
            pings: Mutex::new(HashMap::new()),
            connected: AtomicBool::new(false),
        })))
    }

    pub fn bot(&self) -> Option<&Arc<Bot>> {
        self.0.as_ref()
    }

    pub fn enabled(&self) -> bool {
        self.0.is_some()
    }

    // Queues jobs for the worker, in order. Nothing is queued while the bot is off, so
    // turning it on later doesn't replay the past; a full sync catches it up instead.
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
    Discord(#[from] serenity::Error),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("{0}")]
    App(#[from] crate::error::AppError),
    // Something picked on /admin/discord doesn't exist in the server.
    #[error("{0}")]
    Setup(String),
}

impl BotError {
    // Whether retrying can't help: Discord refused the request itself (missing
    // permissions, unknown channel or user, closed DMs) rather than failing to answer.
    pub fn permanent(&self) -> bool {
        match self {
            BotError::Discord(serenity::Error::Http(err)) => err
                .status_code()
                .is_some_and(|s| s.is_client_error() && s.as_u16() != 429),
            BotError::Discord(serenity::Error::Model(_)) | BotError::Setup(_) => true,
            _ => false,
        }
    }

    // Discord answered that the thing doesn't exist (404).
    pub fn not_found(&self) -> bool {
        matches!(self, BotError::Discord(serenity::Error::Http(err))
            if err.status_code().is_some_and(|s| s.as_u16() == 404))
    }
}

pub type BotResult<T = ()> = Result<T, BotError>;

// Starts the gateway connection, the job worker and the scheduler.
pub fn spawn(state: &AppState) {
    let Some(bot) = state.discord.bot().cloned() else {
        tracing::info!("discord bot is off (DISCORD_BOT_TOKEN / DISCORD_GUILD_ID unset)");
        return;
    };
    tokio::spawn(jobs::run_worker(state.clone(), bot.clone()));
    tokio::spawn(scheduler::run(state.clone()));
    let state = state.clone();
    tokio::spawn(async move {
        // GUILD_MEMBERS is privileged: it has to be enabled on the bot's page.
        let intents = GatewayIntents::GUILDS | GatewayIntents::GUILD_MEMBERS;
        loop {
            let handler = events::Handler {
                state: state.clone(),
                bot: bot.clone(),
            };
            let client = Client::builder(&bot.token, intents)
                .event_handler(handler)
                .await;
            match client {
                Ok(mut client) => {
                    if let Err(err) = client.start().await {
                        tracing::error!(error = %err, "discord gateway stopped");
                    }
                    bot.connected.store(false, Ordering::Relaxed);
                }
                Err(err) => tracing::error!(error = %err, "could not start the discord client"),
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}

// A match's title as its Discord messages show it ("W3: Team A vs Team B"), for jobs
// that outlive the match.
pub fn announcement_title(info: &crate::db::discord::MatchInfo) -> String {
    messages::title(info)
}
