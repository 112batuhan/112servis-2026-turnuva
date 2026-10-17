//! The jobs handlers queue for the bot, and the worker that runs them.

use std::{sync::Arc, time::Duration};

use serde::{Deserialize, Serialize};
use serenity::all::{
    ButtonStyle, CreateActionRow, CreateAllowedMentions, CreateButton, CreateEmbed,
    CreateEmbedFooter, CreateMessage, EditMessage, UserId,
};
use sqlx::postgres::PgListener;
use time::OffsetDateTime;
use uuid::Uuid;

use super::{announcement, channels, members, messages, results, snowflake, Bot, BotResult};
use crate::{
    db::{self, matches::MatchKind, notifications::StaffRole},
    AppState,
};

// How long one job may run. Serenity waits out Discord's rate limits by sleeping, and
// some last minutes (a channel can be renamed twice per 10 minutes), which would hold
// up every job behind it; a job that runs over is retried later instead.
const JOB_TIMEOUT: Duration = Duration::from_secs(60);

// A failed job is tried this many times in all, waiting longer each time.
const MAX_ATTEMPTS: i32 = 8;
const FIRST_RETRY_SECS: i64 = 15;
const MAX_RETRY_SECS: i64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Job {
    // ---- keeping the server in step with the site ----
    // Gives or takes the Verified and Player roles.
    SyncUser {
        user_id: Uuid,
    },
    // The same for an account that may no longer be linked (after relinking).
    SyncMember {
        discord_id: String,
    },
    // Who's in the server, every member's roles and every match channel.
    SyncAll,
    // Creates, updates, archives or reopens a match's private channel.
    SyncMatch {
        match_id: Uuid,
    },
    // Archives the channel of a deleted match.
    ArchiveChannel {
        channel_id: i64,
        kind: MatchKind,
    },
    // Marks a deleted match's public announcement as cancelled. The match is gone, so
    // this carries what the message needs.
    AnnouncementCancelled {
        channel_id: i64,
        message_id: i64,
        title: String,
    },

    // ---- things to announce: each renders its messages and queues a Post for each ----
    TeamFormed {
        team_id: Uuid,
    },
    TeamDisbanded {
        name: String,
        players: Vec<Uuid>,
        by_host: bool,
    },
    ProposalMade {
        match_id: Uuid,
        team_id: Uuid,
        #[serde(with = "time::serde::rfc3339")]
        starts_at: OffsetDateTime,
    },
    Rescheduled {
        match_id: Uuid,
        actor_id: Uuid,
        #[serde(with = "time::serde::rfc3339::option")]
        old_starts_at: Option<OffsetDateTime>,
        #[serde(with = "time::serde::rfc3339")]
        new_starts_at: OffsetDateTime,
    },
    StaffAnswered {
        match_id: Uuid,
        user_id: Uuid,
        role: StaffRole,
        declined: bool,
    },
    StaffChanged {
        match_id: Uuid,
        role: StaffRole,
        user_id: Option<Uuid>,
        by: Uuid,
    },
    // A host moved the match.
    TimeSet {
        match_id: Uuid,
    },
    // DMs the referee and streamer the confirm_time questions about the match's
    // current time (Rescheduled sends its own).
    ConfirmTime {
        match_id: Uuid,
    },
    RoomAdded {
        match_id: Uuid,
        mp_id: i64,
    },
    Concluded {
        match_id: Uuid,
    },
    Reopened {
        match_id: Uuid,
    },
    Reminder {
        match_id: Uuid,
    },
    Announce {
        match_id: Uuid,
    },
    PostResults {
        match_id: Uuid,
    },
    DeleteResults {
        match_id: Uuid,
    },
    Ping {
        match_id: Uuid,
        user_id: Uuid,
        by: Uuid,
    },
    MemberLeft {
        user_id: Uuid,
    },
    MemberReturned {
        user_id: Uuid,
    },
    RankOverLimit {
        team_id: Uuid,
        user_id: Uuid,
        old_rank: Option<i64>,
        new_rank: Option<i64>,
        team_value: f64,
    },

    // ---- one rendered message to one place ----
    Post {
        to: Target,
        // Boxed: a message is far bigger than any other job.
        message: Box<OutMessage>,
        // The match whose channel this tells that it was concluded or reopened: the
        // message replaces the last such one there, so a reopened match doesn't keep
        // its old result in the channel.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        status_of: Option<Uuid>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", content = "id", rename_all = "snake_case")]
pub enum Target {
    Channel(i64),
    // Skipped when the match has no channel.
    MatchChannel(Uuid),
    // A direct message to a Discord account.
    User(String),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutMessage {
    pub content: String,
    // The accounts the message may ping; any other mention stays silent.
    #[serde(default)]
    pub mentions: Vec<String>,
    #[serde(default)]
    pub embed: Option<Embed>,
    // More images for the embed, after its own: each goes out as an embed with only the
    // main one's url and the image, which Discord shows as one gallery (up to 4 images).
    #[serde(default)]
    pub gallery: Vec<String>,
    #[serde(default)]
    pub buttons: Vec<Button>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Embed {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub color: Option<u32>,
    #[serde(default)]
    pub fields: Vec<(String, String, bool)>,
    pub footer: Option<String>,
    pub thumbnail: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Button {
    pub custom_id: String,
    pub label: String,
    pub danger: bool,
}

impl Embed {
    pub fn new(title: impl Into<String>, color: u32) -> Self {
        Self {
            title: Some(title.into()),
            color: Some(color),
            ..Self::default()
        }
    }

    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn field(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        inline: bool,
    ) -> Self {
        self.fields.push((name.into(), value.into(), inline));
        self
    }

    pub fn build(&self) -> CreateEmbed {
        let mut e = CreateEmbed::new();
        if let Some(t) = &self.title {
            e = e.title(t);
        }
        if let Some(d) = &self.description {
            e = e.description(d);
        }
        if let Some(u) = &self.url {
            e = e.url(u);
        }
        if let Some(c) = self.color {
            e = e.color(c);
        }
        for (name, value, inline) in &self.fields {
            e = e.field(name, value, *inline);
        }
        if let Some(f) = &self.footer {
            e = e.footer(CreateEmbedFooter::new(f));
        }
        if let Some(t) = &self.thumbnail {
            e = e.thumbnail(t);
        }
        if let Some(i) = &self.image {
            e = e.image(i);
        }
        e
    }
}

impl OutMessage {
    pub fn text(content: impl Into<String>, mentions: Vec<String>) -> Self {
        Self {
            content: content.into(),
            mentions,
            ..Self::default()
        }
    }

    // An embed, with whoever it may ping mentioned above it: mentions inside an embed
    // never ping.
    pub fn embed(embed: Embed, mentions: Vec<String>) -> Self {
        Self {
            content: mentions
                .iter()
                .map(|id| format!("<@{id}>"))
                .collect::<Vec<_>>()
                .join(" "),
            mentions,
            embed: Some(embed),
            ..Self::default()
        }
    }

    // The embed and its gallery.
    fn embeds(&self) -> Vec<CreateEmbed> {
        let Some(embed) = &self.embed else {
            return Vec::new();
        };
        let mut all = vec![embed.build()];
        for image in &self.gallery {
            let mut e = CreateEmbed::new().image(image);
            if let Some(url) = &embed.url {
                e = e.url(url);
            }
            all.push(e);
        }
        all
    }

    // The same message as an edit. Edits never ping, whatever they mention.
    pub fn build_edit(&self) -> EditMessage {
        EditMessage::new()
            .content(&self.content)
            .allowed_mentions(CreateAllowedMentions::new())
            .embeds(self.embeds())
    }

    pub fn build(&self) -> CreateMessage {
        let users: Vec<UserId> = self
            .mentions
            .iter()
            .filter_map(|id| snowflake::user(id))
            .collect();
        let mut m = CreateMessage::new()
            .content(&self.content)
            .allowed_mentions(CreateAllowedMentions::new().users(users));
        if self.embed.is_some() {
            m = m.embeds(self.embeds());
        }
        if !self.buttons.is_empty() {
            let buttons = self
                .buttons
                .iter()
                .map(|b| {
                    CreateButton::new(&b.custom_id)
                        .label(&b.label)
                        .style(if b.danger {
                            ButtonStyle::Danger
                        } else {
                            ButtonStyle::Success
                        })
                })
                .collect();
            m = m.components(vec![CreateActionRow::Buttons(buttons)]);
        }
        m
    }
}

// Runs queued jobs oldest first, waking up when a transaction queues more (pg_notify in
// push_jobs) and when a retry comes due.
pub async fn run_worker(state: AppState, bot: Arc<Bot>) {
    let mut listener = loop {
        match PgListener::connect_with(&state.db).await {
            Ok(mut l) => match l.listen("discord_outbox").await {
                Ok(()) => break l,
                Err(err) => tracing::error!(error = %err, "discord worker could not listen"),
            },
            Err(err) => tracing::error!(error = %err, "discord worker could not connect"),
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    };
    loop {
        bot.until_active(true).await;
        // A job that's running when the bot is switched off still finishes.
        while bot.active() {
            match db::discord::next_job(&state.db).await {
                Ok(Some(row)) => process(&state, &bot, row).await,
                Ok(None) => break,
                Err(err) => {
                    tracing::error!(error = %err, "discord worker could not read the queue");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                    break;
                }
            }
        }
        let wait = db::discord::next_retry_in(&state.db)
            .await
            .ok()
            .flatten()
            .map_or(60.0, |s| s.clamp(1.0, 60.0));
        tokio::select! {
            notified = listener.recv() => {
                if let Err(err) = notified {
                    tracing::warn!(error = %err, "discord worker lost its listener");
                    tokio::time::sleep(Duration::from_secs(5)).await;
                }
            }
            _ = tokio::time::sleep(Duration::from_secs_f64(wait)) => {}
            _ = bot.until_active(false) => {}
        }
    }
}

async fn process(state: &AppState, bot: &Bot, row: db::discord::OutboxRow) {
    let job: Job = match serde_json::from_value(row.job.0.clone()) {
        Ok(job) => job,
        Err(err) => {
            tracing::error!(error = %err, job = %row.job.0, "dropping an unreadable discord job");
            let _ = db::discord::finish_job(&state.db, row.id).await;
            return;
        }
    };
    // A full sync makes a call per member and channel, so it may take a while.
    let result = if matches!(job, Job::SyncAll) {
        run(state, bot, &job).await
    } else {
        tokio::time::timeout(JOB_TIMEOUT, run(state, bot, &job))
            .await
            .unwrap_or(Err(super::BotError::Timeout))
    };
    let done = match &result {
        Ok(()) => true,
        Err(err) if err.permanent() => {
            tracing::warn!(error = %err, ?job, "discord job failed for good");
            true
        }
        Err(err) if row.attempts + 1 >= MAX_ATTEMPTS => {
            tracing::error!(error = %err, ?job, "discord job failed too often, dropping it");
            true
        }
        Err(err) => {
            tracing::warn!(error = %err, ?job, attempt = row.attempts + 1, "discord job failed, retrying");
            false
        }
    };
    let saved = if done {
        db::discord::finish_job(&state.db, row.id).await
    } else {
        let wait = (FIRST_RETRY_SECS << row.attempts.min(10)).min(MAX_RETRY_SECS);
        db::discord::retry_job(&state.db, row.id, wait).await
    };
    if let Err(err) = saved {
        tracing::error!(error = %err, "could not update the discord queue");
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run(state: &AppState, bot: &Bot, job: &Job) -> BotResult {
    match job {
        Job::SyncUser { user_id } => members::sync_user(state, bot, *user_id).await,
        Job::SyncMember { discord_id } => members::sync_member(state, bot, discord_id).await,
        Job::SyncAll => {
            members::reconcile(state, bot).await?;
            channels::sync_all(state).await
        }
        Job::SyncMatch { match_id } => channels::sync_match(state, bot, *match_id).await,
        Job::ArchiveChannel { channel_id, kind } => {
            channels::archive_orphan(state, bot, *channel_id, *kind).await
        }
        Job::AnnouncementCancelled {
            channel_id,
            message_id,
            title,
        } => announcement::cancelled(bot, *channel_id, *message_id, title).await,
        Job::Announce { match_id } => announcement::post(state, bot, *match_id).await,
        Job::PostResults { match_id } => results::post(state, bot, *match_id).await,
        Job::DeleteResults { match_id } => results::delete(state, bot, *match_id).await,
        Job::Post {
            to,
            message,
            status_of,
        } => post(state, bot, to, message, *status_of).await,
        event => {
            let posts = messages::render(state, event).await?;
            let status_of = match event {
                Job::Concluded { match_id } | Job::Reopened { match_id } => Some(*match_id),
                _ => None,
            };
            let jobs: Vec<Job> = posts
                .into_iter()
                .map(|(to, message)| Job::Post {
                    status_of: status_of.filter(|_| matches!(to, Target::MatchChannel(_))),
                    to,
                    message: Box::new(message),
                })
                .collect();
            state.discord.enqueue(&state.db, &jobs).await?;
            Ok(())
        }
    }
}

async fn post(
    state: &AppState,
    bot: &Bot,
    to: &Target,
    message: &OutMessage,
    status_of: Option<Uuid>,
) -> BotResult {
    let channel = match to {
        Target::Channel(id) => snowflake::channel(*id),
        Target::MatchChannel(match_id) => {
            match db::discord::channel_of(&state.db, *match_id).await? {
                Some(id) => snowflake::channel(id),
                None => return Ok(()),
            }
        }
        Target::User(discord_id) => {
            let Some(user) = snowflake::user(discord_id) else {
                return Ok(());
            };
            user.create_dm_channel(&bot.http).await?.id
        }
    };
    let sent = channel.send_message(&bot.http, message.build()).await?;
    if let Some(match_id) = status_of {
        let previous =
            db::discord::swap_status_message(&state.db, match_id, sent.id.get() as i64).await?;
        if let Some(previous) = previous {
            channels::delete_message(bot, channel.get() as i64, previous).await?;
        }
    }
    Ok(())
}
