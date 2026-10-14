//! What the bot hears from the gateway.

use std::sync::{atomic::Ordering, Arc};

use serenity::{
    all::{Context, EventHandler, GuildId, Interaction, Member, Ready, ResumedEvent, User},
    async_trait,
};

use super::{commands, jobs::Job, members, Bot};
use crate::AppState;

pub struct Handler {
    pub state: AppState,
    pub bot: Arc<Bot>,
}

#[async_trait]
impl EventHandler for Handler {
    // On every (re)connect: register the slash commands and catch up on whatever
    // happened while the bot was away.
    async fn ready(&self, _ctx: Context, ready: Ready) {
        self.bot.connected.store(true, Ordering::Relaxed);
        let _ = self.bot.user_id.set(ready.user.id);
        tracing::info!(bot = %ready.user.name, "discord bot connected");
        if let Err(err) = self
            .bot
            .guild_id
            .set_commands(&self.bot.http, commands::definitions())
            .await
        {
            tracing::error!(error = %err, "could not register the slash commands");
        }
        if let Err(err) = self
            .state
            .discord
            .enqueue(&self.state.db, &[Job::SyncAll])
            .await
        {
            tracing::error!(error = %err, "could not queue the discord sync");
        }
    }

    async fn resume(&self, _ctx: Context, _event: ResumedEvent) {
        self.bot.connected.store(true, Ordering::Relaxed);
    }

    async fn guild_member_addition(&self, _ctx: Context, member: Member) {
        if member.guild_id != self.bot.guild_id || member.user.bot {
            return;
        }
        if let Err(err) = members::joined(&self.state, &self.bot, &member).await {
            tracing::error!(error = %err, "could not handle a member joining");
        }
    }

    async fn guild_member_removal(
        &self,
        _ctx: Context,
        guild_id: GuildId,
        user: User,
        _member: Option<Member>,
    ) {
        if guild_id != self.bot.guild_id || user.bot {
            return;
        }
        if let Err(err) = members::left(&self.state, &user.id.to_string()).await {
            tracing::error!(error = %err, "could not handle a member leaving");
        }
    }

    async fn interaction_create(&self, _ctx: Context, interaction: Interaction) {
        if interaction
            .guild_id()
            .is_some_and(|g| g != self.bot.guild_id)
        {
            return;
        }
        commands::handle(&self.state, &self.bot, interaction).await;
    }
}
