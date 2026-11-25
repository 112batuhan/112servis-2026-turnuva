//! A bracket match's public announcement in the announcement channel. Posted 15 minutes
//! before the match, pinging its subscribers (scheduler.rs queues Job::Announce), then
//! kept up to date by every SyncMatch like the match card: the live score while it's
//! played, the new time when it's moved. A moved match is announced again before its
//! new time, and the old message goes then. A concluded match loses its announcement,
//! forfeits included, since its result goes to the results channel; a deleted one keeps
//! it, marked as cancelled (Job::AnnouncementCancelled).

use serenity::all::{EditMessage, MessageId};
use uuid::Uuid;

use super::{
    channels,
    jobs::{Embed, Job, OutMessage, Target},
    messages, snowflake, Bot, BotError, BotResult,
};
use crate::{
    db::{
        self,
        discord::{MatchDiscord, MatchInfo, MatchPlayer},
        matches::MatchKind,
    },
    AppState,
};

// What a rendered announcement is compared by, to skip edits that change nothing.
fn key(message: &OutMessage) -> String {
    serde_json::to_string(&(&message.content, &message.embed, &message.gallery)).unwrap_or_default()
}

// Job::Announce: posts the announcement for the match's current time, pinging its
// subscribers, then deletes the one it had for an earlier time.
pub async fn post(state: &AppState, bot: &Bot, match_id: Uuid) -> BotResult {
    let settings = db::discord::settings(&state.db).await?;
    let Some(channel) = settings.match_announce_channel_id else {
        return Ok(());
    };
    let Some(info) = db::discord::match_info(&state.db, match_id).await? else {
        return Ok(());
    };
    if info.kind != MatchKind::Bracket || info.concluded_at.is_some() || info.starts_at.is_none() {
        return Ok(());
    }
    let stored = db::discord::match_discord(&state.db, match_id).await?;
    // Moved again before this ran: the scheduler queues another one for the new time.
    if stored.announced_for != info.starts_at {
        return Ok(());
    }
    // Paused (jobs::post): this time's announcement is skipped, not posted later.
    if db::discord::channels_paused(&state.db).await? {
        return Ok(());
    }
    let players = db::discord::match_players(&state.db, match_id).await?;
    let message = messages::announcement(state, &info, &players, stored.announced_for);
    let synced = key(&message);

    let subscribers = db::discord::subscriber_discord_ids(&state.db, match_id).await?;
    let (first, rest) = messages::mention_chunks(&subscribers);
    let with_pings = OutMessage {
        content: format!("{} {}", message.content, messages::mentions(&first))
            .trim_end()
            .to_string(),
        mentions: first,
        ..message
    };
    let sent = snowflake::channel(channel)
        .send_message(&bot.http, with_pings.build())
        .await?;
    db::discord::set_announcement(
        &state.db,
        match_id,
        Some((channel, sent.id.get() as i64)),
        Some(&synced),
    )
    .await?;
    // The pings that didn't fit, each as a plain message of its own.
    let extra: Vec<Job> = rest
        .into_iter()
        .map(|ids| Job::Post {
            to: Target::Channel(channel),
            message: Box::new(OutMessage::text(messages::mentions(&ids), ids)),
            status_of: None,
        })
        .collect();
    state.discord.enqueue(&state.db, &extra).await?;

    if let (Some(old_channel), Some(old_message)) =
        (stored.announce_channel_id, stored.announce_message_id)
    {
        channels::delete_message(bot, old_channel, old_message).await?;
    }
    Ok(())
}

// Part of SyncMatch: brings the match's announcement, if it has one, in line with it.
pub async fn sync(
    state: &AppState,
    bot: &Bot,
    info: &MatchInfo,
    players: &[MatchPlayer],
    stored: &MatchDiscord,
) -> BotResult {
    let (Some(channel), Some(message_id)) =
        (stored.announce_channel_id, stored.announce_message_id)
    else {
        return Ok(());
    };
    // Concluded, played or forfeited: the results channel has it now.
    if info.concluded_at.is_some() {
        channels::delete_message(bot, channel, message_id).await?;
        return db::discord::set_announcement(&state.db, info.id, None, None)
            .await
            .map_err(Into::into);
    }
    let message = messages::announcement(state, info, players, stored.announced_for);
    let synced = key(&message);
    if stored.announce_synced.as_deref() == Some(synced.as_str()) {
        return Ok(());
    }
    match snowflake::channel(channel)
        .edit_message(
            &bot.http,
            MessageId::new(message_id as u64),
            message.build_edit(),
        )
        .await
        .map_err(BotError::from)
    {
        Ok(_) => {
            db::discord::set_announcement(
                &state.db,
                info.id,
                Some((channel, message_id)),
                Some(&synced),
            )
            .await?
        }
        // Someone deleted it: there's nothing left to keep up to date.
        Err(err) if err.not_found() => {
            db::discord::set_announcement(&state.db, info.id, None, None).await?
        }
        Err(err) => return Err(err),
    }
    Ok(())
}

// Job::AnnouncementCancelled: the match was deleted, so its announcement says so. The
// match is gone by now, so the job carries what the message needs.
pub async fn cancelled(bot: &Bot, channel: i64, message: i64, title: &str) -> BotResult {
    let embed = Embed {
        title: Some(title.to_string()),
        description: Some("Bu maç iptal edildi.".into()),
        color: Some(messages::COLOR_DONE),
        ..Embed::default()
    };
    let edit = EditMessage::new()
        .content(format!("❌ **{title}** iptal edildi."))
        .allowed_mentions(serenity::all::CreateAllowedMentions::new())
        .embeds(vec![embed.build()]);
    match snowflake::channel(channel)
        .edit_message(&bot.http, MessageId::new(message as u64), edit)
        .await
        .map_err(BotError::from)
    {
        Ok(_) => Ok(()),
        Err(err) if err.not_found() => Ok(()),
        Err(err) => Err(err),
    }
}
