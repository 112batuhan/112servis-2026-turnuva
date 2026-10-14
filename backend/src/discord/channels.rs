//! Each match's private channel: its players, referee and streamer can see it. A bracket
//! match gets one once it has both teams and a time, a qualifier lobby once a team signs
//! up. A concluded match's channel turns read-only and moves to the archive category.

use serenity::all::{
    ChannelId, ChannelType, CreateChannel, EditChannel, MessageId, PermissionOverwrite,
    PermissionOverwriteType, Permissions, RoleId, UserId,
};
use uuid::Uuid;

use super::{jobs::Job, messages, snowflake, Bot, BotError, BotResult};
use crate::{
    db::{
        self,
        discord::{MatchDiscord, MatchInfo, MatchPlayer, Settings},
        matches::MatchKind,
    },
    AppState,
};

// Discord's limit on channels in a category.
const CATEGORY_LIMIT: usize = 50;

fn active_perms() -> Permissions {
    Permissions::VIEW_CHANNEL
        | Permissions::SEND_MESSAGES
        | Permissions::READ_MESSAGE_HISTORY
        | Permissions::EMBED_LINKS
        | Permissions::ATTACH_FILES
        | Permissions::ADD_REACTIONS
}

fn read_only_deny() -> Permissions {
    Permissions::SEND_MESSAGES
        | Permissions::ADD_REACTIONS
        | Permissions::SEND_MESSAGES_IN_THREADS
        | Permissions::CREATE_PUBLIC_THREADS
        | Permissions::CREATE_PRIVATE_THREADS
}

fn member_overwrite(user: UserId, archived: bool) -> PermissionOverwrite {
    PermissionOverwrite {
        allow: if archived {
            Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY
        } else {
            active_perms()
        },
        deny: if archived {
            read_only_deny()
        } else {
            Permissions::empty()
        },
        kind: PermissionOverwriteType::Member(user),
    }
}

// Hidden from everyone but the bot and the match's people. Hosts see it as admins.
async fn overwrites(
    bot: &Bot,
    members: &[UserId],
    archived: bool,
) -> BotResult<Vec<PermissionOverwrite>> {
    let me = bot.user_id().await?;
    let mut list = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(bot.guild_id.get())),
        },
        PermissionOverwrite {
            allow: active_perms(),
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(me),
        },
    ];
    list.extend(
        members
            .iter()
            .filter(|&&u| u != me)
            .map(|&u| member_overwrite(u, archived)),
    );
    Ok(list)
}

// "w1-sokak-kedileri-vs-tatli-krizi", "lobi-3". Discord would lowercase and dash it the
// same way; doing it here keeps the name comparable with what Discord stores.
fn channel_name(info: &MatchInfo) -> String {
    let raw = match info.kind {
        MatchKind::Bracket => format!(
            "{} {} vs {}",
            messages::code(info),
            info.team1_name.as_deref().unwrap_or("tbd"),
            info.team2_name.as_deref().unwrap_or("tbd"),
        ),
        MatchKind::Qualifier => messages::code(info),
    };
    let mut name = String::new();
    for c in raw.chars().flat_map(char::to_lowercase) {
        if c.is_alphanumeric() || c == '_' {
            name.push(c);
        } else if (c.is_whitespace() || c == '-') && !name.ends_with('-') && !name.is_empty() {
            name.push('-');
        }
    }
    let name = name.trim_end_matches('-');
    name.chars().take(90).collect()
}

// Where the channel belongs: the open or the archive category of its kind.
fn base_category(settings: &Settings, kind: MatchKind, archived: bool) -> Option<i64> {
    match (kind, archived) {
        (MatchKind::Bracket, false) => settings.bracket_category_id,
        (MatchKind::Bracket, true) => settings.bracket_archive_category_id,
        (MatchKind::Qualifier, false) => settings.qualifier_category_id,
        (MatchKind::Qualifier, true) => settings.qualifier_archive_category_id,
    }
}

fn should_open(info: &MatchInfo) -> bool {
    if info.concluded_at.is_some() {
        return false;
    }
    match info.kind {
        MatchKind::Bracket => {
            info.team1_id.is_some() && info.team2_id.is_some() && info.starts_at.is_some()
        }
        MatchKind::Qualifier => info.entries > 0,
    }
}

// Brings a match's channel in line with the match, then its pinned card. What was
// applied is remembered, so an unchanged match costs no API calls.
pub async fn sync_match(state: &AppState, bot: &Bot, match_id: Uuid) -> BotResult {
    let Some(info) = db::discord::match_info(&state.db, match_id).await? else {
        return Ok(());
    };
    let stored = db::discord::match_discord(&state.db, match_id).await?;
    let players = db::discord::match_players(&state.db, match_id).await?;
    let Some(channel) = sync_channel(state, bot, &info, &players, &stored).await? else {
        return Ok(());
    };
    // A channel made just now has no card yet, whatever the old row said.
    let stored = if Some(channel) == stored.channel_id {
        stored
    } else {
        MatchDiscord::default()
    };
    sync_card(state, bot, &info, &players, channel, &stored).await
}

// The channel's name, category and who can see it. Returns the channel, if the match
// has one.
async fn sync_channel(
    state: &AppState,
    bot: &Bot,
    info: &MatchInfo,
    players: &[MatchPlayer],
    stored: &MatchDiscord,
) -> BotResult<Option<i64>> {
    let match_id = info.id;
    if stored.channel_id.is_none() && !should_open(info) {
        return Ok(None);
    }
    let archived = info.concluded_at.is_some();
    let settings = db::discord::settings(&state.db).await?;
    let base = base_category(&settings, info.kind, archived);

    let mut members: Vec<UserId> = players
        .iter()
        .filter(|p| p.in_guild == Some(true))
        .filter_map(|p| snowflake::user(p.discord_id.as_deref()?))
        .collect();
    for (id, in_guild) in [
        (&info.referee_discord_id, info.referee_in_guild),
        (&info.streamer_discord_id, info.streamer_in_guild),
    ] {
        if in_guild == Some(true) {
            members.extend(id.as_deref().and_then(snowflake::user));
        }
    }
    members.sort();
    members.dedup();

    let name = channel_name(info);
    let key = format!(
        "{name}|{archived}|{base:?}|{}",
        members
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",")
    );
    if stored.synced.as_deref() == Some(key.as_str()) {
        return Ok(stored.channel_id);
    }
    let perms = overwrites(bot, &members, archived).await?;

    if let Some(id) = stored.channel_id {
        let channel = snowflake::channel(id);
        match channel.to_channel(&bot.http).await {
            Ok(current) => {
                let current = current.guild();
                let mut edit = EditChannel::new().permissions(perms);
                if current.as_ref().is_none_or(|c| c.name != name) {
                    edit = edit.name(&name);
                }
                if let Some(base) = base {
                    let parent = current.as_ref().and_then(|c| c.parent_id);
                    if !in_base(state, base, parent).await? {
                        edit = edit.category(Some(category_with_room(state, bot, base).await?));
                    }
                }
                channel.edit(&bot.http, edit).await?;
                db::discord::set_channel(&state.db, match_id, id, &key).await?;
                return Ok(Some(id));
            }
            Err(err) => {
                let err = BotError::from(err);
                if !err.not_found() {
                    return Err(err);
                }
                // Deleted by hand: open a new one if the match still needs it.
                db::discord::clear_channel(&state.db, match_id).await?;
                if !should_open(info) {
                    return Ok(None);
                }
            }
        }
    }

    // A channel is only created once its category has been picked.
    let Some(base) = base else {
        return Ok(None);
    };
    let category = category_with_room(state, bot, base).await?;
    let created = bot
        .guild_id
        .create_channel(
            &bot.http,
            CreateChannel::new(&name)
                .kind(ChannelType::Text)
                .category(category)
                .permissions(perms),
        )
        .await?;
    let id = created.id.get() as i64;
    db::discord::set_channel(&state.db, match_id, id, &key).await?;
    Ok(Some(id))
}

// The pinned card at the top of the channel: posted (pinging everyone in the match)
// with the channel, edited whenever the match changes, and posted again if someone
// deleted it.
async fn sync_card(
    state: &AppState,
    bot: &Bot,
    info: &MatchInfo,
    players: &[MatchPlayer],
    channel_id: i64,
    stored: &MatchDiscord,
) -> BotResult {
    let card = messages::match_card(state, info, players);
    let key = serde_json::to_string(&(&card.content, &card.embed)).unwrap_or_default();
    if stored.card_synced.as_deref() == Some(key.as_str()) {
        return Ok(());
    }
    let channel = snowflake::channel(channel_id);
    if let Some(id) = stored.card_message_id {
        match channel
            .edit_message(&bot.http, MessageId::new(id as u64), card.build_edit())
            .await
        {
            Ok(_) => {
                db::discord::set_card(&state.db, info.id, id, &key).await?;
                return Ok(());
            }
            Err(err) => {
                let err = BotError::from(err);
                if !err.not_found() {
                    return Err(err);
                }
            }
        }
    }
    // A finished match without a card (from before cards, or deleted) keeps going
    // without one rather than pinging everyone in the archive.
    if info.concluded_at.is_some() {
        return Ok(());
    }
    let message = channel.send_message(&bot.http, card.build()).await?;
    db::discord::set_card(&state.db, info.id, message.id.get() as i64, &key).await?;
    // Pinning needs Manage Messages, which the bot may not have been given; the card
    // still works unpinned.
    if let Err(err) = channel.pin(&bot.http, message.id).await {
        tracing::warn!(error = %err, match_id = %info.id, "could not pin the match card");
    }
    Ok(())
}

// Whether a channel's category is the base one or one of its overflows.
async fn in_base(state: &AppState, base: i64, parent: Option<ChannelId>) -> BotResult<bool> {
    let Some(parent) = parent.map(|p| p.get() as i64) else {
        return Ok(false);
    };
    Ok(parent == base
        || db::discord::overflow_categories(&state.db, base)
            .await?
            .contains(&parent))
}

// The base category if it has room, otherwise one of the categories made when it filled
// up, or a new one next to it with the same permissions.
async fn category_with_room(state: &AppState, bot: &Bot, base: i64) -> BotResult<ChannelId> {
    let channels = bot.guild_id.channels(&bot.http).await?;
    let count = |cat: ChannelId| {
        channels
            .values()
            .filter(|c| c.parent_id == Some(cat))
            .count()
    };
    let base_id = snowflake::channel(base);
    let Some(base_channel) = channels.get(&base_id) else {
        return Err(BotError::Setup(format!("category {base} doesn't exist")));
    };
    if count(base_id) < CATEGORY_LIMIT {
        return Ok(base_id);
    }
    let overflows = db::discord::overflow_categories(&state.db, base).await?;
    for &id in &overflows {
        let id = snowflake::channel(id);
        if channels.contains_key(&id) && count(id) < CATEGORY_LIMIT {
            return Ok(id);
        }
    }
    let n = overflows.len() + 2;
    let created = bot
        .guild_id
        .create_channel(
            &bot.http,
            CreateChannel::new(format!("{} {n}", base_channel.name))
                .kind(ChannelType::Category)
                .position(base_channel.position.saturating_add(n as u16 - 1))
                .permissions(base_channel.permission_overwrites.clone()),
        )
        .await?;
    db::discord::add_overflow_category(&state.db, base, created.id.get() as i64).await?;
    Ok(created.id)
}

// The channel of a deleted match: everyone in it keeps reading it, nobody writes, and
// it goes to the archive.
pub async fn archive_orphan(
    state: &AppState,
    bot: &Bot,
    channel_id: i64,
    kind: MatchKind,
) -> BotResult {
    let channel = snowflake::channel(channel_id);
    let current = match channel.to_channel(&bot.http).await {
        Ok(c) => c.guild(),
        Err(err) => {
            let err = BotError::from(err);
            return if err.not_found() { Ok(()) } else { Err(err) };
        }
    };
    let Some(current) = current else {
        return Ok(());
    };
    let me = bot.user_id().await?;
    let perms: Vec<PermissionOverwrite> = current
        .permission_overwrites
        .iter()
        .map(|o| match o.kind {
            PermissionOverwriteType::Member(u) if u != me => member_overwrite(u, true),
            _ => o.clone(),
        })
        .collect();
    let mut edit = EditChannel::new().permissions(perms);
    let settings = db::discord::settings(&state.db).await?;
    if let Some(base) = base_category(&settings, kind, true) {
        if !in_base(state, base, current.parent_id).await? {
            edit = edit.category(Some(category_with_room(state, bot, base).await?));
        }
    }
    channel.edit(&bot.http, edit).await?;
    Ok(())
}

// Queues a sync of every match that has a channel or should have one.
pub async fn sync_all(state: &AppState) -> BotResult {
    let jobs: Vec<Job> = db::discord::syncable_matches(&state.db)
        .await?
        .into_iter()
        .map(|match_id| Job::SyncMatch { match_id })
        .collect();
    state.discord.enqueue(&state.db, &jobs).await?;
    Ok(())
}
