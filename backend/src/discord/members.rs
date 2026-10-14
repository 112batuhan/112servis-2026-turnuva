//! Who's in the server, and the Verified and Player roles.

use std::collections::HashMap;

use serenity::all::{AddMember, Member, RoleId, UserId};
use uuid::Uuid;

use super::{jobs::Job, snowflake, Bot, BotResult};
use crate::{
    db::{self, discord::LinkedAccount},
    AppState,
};

// The roles the bot hands out, as far as hosts have picked them.
struct Roles {
    verified: Option<RoleId>,
    player: Option<RoleId>,
}

impl Roles {
    async fn load(state: &AppState) -> sqlx::Result<Self> {
        let s = db::discord::settings(&state.db).await?;
        Ok(Self {
            verified: s.verified_role_id.map(snowflake::role),
            player: s.player_role_id.map(snowflake::role),
        })
    }

    fn managed(&self) -> impl Iterator<Item = RoleId> + '_ {
        self.verified.iter().chain(self.player.iter()).copied()
    }

    // Verified for a linked account, Player too once it's in a team.
    fn wanted(&self, account: Option<&LinkedAccount>) -> Vec<RoleId> {
        let Some(account) = account else {
            return Vec::new();
        };
        let mut roles: Vec<RoleId> = self.verified.into_iter().collect();
        if account.in_team {
            roles.extend(self.player);
        }
        roles
    }
}

async fn apply(
    bot: &Bot,
    roles: &Roles,
    user: UserId,
    current: &[RoleId],
    wanted: &[RoleId],
) -> BotResult {
    for role in roles.managed() {
        let has = current.contains(&role);
        let want = wanted.contains(&role);
        if want && !has {
            bot.http
                .add_member_role(bot.guild_id, user, role, Some("turnuva sitesi"))
                .await?;
        } else if has && !want {
            bot.http
                .remove_member_role(bot.guild_id, user, role, Some("turnuva sitesi"))
                .await?;
        }
    }
    Ok(())
}

// Adds a linking account to the server with its OAuth token (the guilds.join scope).
// Already being in it is fine.
pub async fn add_to_guild(bot: &Bot, discord_id: &str, access_token: &str) -> BotResult {
    let Some(user) = snowflake::user(discord_id) else {
        return Ok(());
    };
    bot.guild_id
        .add_member(&*bot.http, user, AddMember::new(access_token.to_string()))
        .await?;
    Ok(())
}

pub async fn sync_user(state: &AppState, bot: &Bot, user_id: Uuid) -> BotResult {
    match db::discord::account_of(&state.db, user_id).await? {
        Some((discord_id, _)) => sync_member(state, bot, &discord_id).await,
        None => Ok(()),
    }
}

// Gives or takes the roles of one account, linked or not.
pub async fn sync_member(state: &AppState, bot: &Bot, discord_id: &str) -> BotResult {
    let Some(user) = snowflake::user(discord_id) else {
        return Ok(());
    };
    let roles = Roles::load(state).await?;
    if roles.managed().next().is_none() {
        return Ok(());
    }
    let member = match bot.guild_id.member(&bot.http, user).await {
        Ok(member) => member,
        Err(err) => {
            let err = super::BotError::from(err);
            return if err.not_found() { Ok(()) } else { Err(err) };
        }
    };
    let account = db::discord::find_linked(&state.db, discord_id).await?;
    apply(
        bot,
        &roles,
        user,
        &member.roles,
        &roles.wanted(account.as_ref()),
    )
    .await
}

// Brings the site in line with the whole server: who's in it (catching up on joins and
// leaves missed while the bot was offline) and everyone's roles.
pub async fn reconcile(state: &AppState, bot: &Bot) -> BotResult {
    let mut members = Vec::new();
    let mut after = None;
    loop {
        let page = bot.guild_id.members(&bot.http, Some(1000), after).await?;
        let full = page.len() == 1000;
        after = page.last().map(|m| m.user.id);
        members.extend(page.into_iter().filter(|m| !m.user.bot));
        if !full {
            break;
        }
    }
    let present: HashMap<String, Vec<RoleId>> = members
        .into_iter()
        .map(|m| (m.user.id.to_string(), m.roles))
        .collect();

    let accounts = db::discord::linked_accounts(&state.db).await?;
    for account in &accounts {
        let now_in = present.contains_key(&account.discord_id);
        if account.in_guild == Some(now_in) {
            continue;
        }
        db::discord::set_in_guild(&state.db, &account.discord_id, now_in).await?;
        match (account.in_guild, now_in) {
            (Some(true), false) => on_left(state, account.user_id).await?,
            (_, true) => on_back(state, account.user_id, account.in_guild == Some(false)).await?,
            _ => {}
        }
    }

    let roles = Roles::load(state).await?;
    if roles.managed().next().is_none() {
        return Ok(());
    }
    let by_discord: HashMap<&str, &LinkedAccount> = accounts
        .iter()
        .map(|a| (a.discord_id.as_str(), a))
        .collect();
    // A member Discord won't let us change (a role above the bot's, say) is skipped, so
    // one of them doesn't stop everyone else's roles and the channel sync after this.
    // Anything worth retrying still fails the job.
    let mut skipped = 0;
    for (id, current) in &present {
        let Some(user) = snowflake::user(id) else {
            continue;
        };
        let wanted = roles.wanted(by_discord.get(id.as_str()).copied());
        match apply(bot, &roles, user, current, &wanted).await {
            Ok(()) => {}
            Err(err) if err.permanent() => {
                tracing::warn!(error = %err, discord_id = %id, "could not change a member's roles");
                skipped += 1;
            }
            Err(err) => return Err(err),
        }
    }
    if skipped > 0 {
        tracing::warn!(
            skipped,
            "some members' roles weren't changed; check that the bot has Manage Roles and its \
             role is above Verified and Player"
        );
    }
    Ok(())
}

// Someone joined the server: if they've linked the account, they get their roles and
// are added to their matches' channels.
pub async fn joined(state: &AppState, bot: &Bot, member: &Member) -> BotResult {
    let discord_id = member.user.id.to_string();
    let Some((user_id, was)) = db::discord::set_in_guild(&state.db, &discord_id, true).await?
    else {
        return Ok(());
    };
    on_back(state, user_id, was == Some(false)).await?;
    let roles = Roles::load(state).await?;
    let account = db::discord::find_linked(&state.db, &discord_id).await?;
    apply(
        bot,
        &roles,
        member.user.id,
        &member.roles,
        &roles.wanted(account.as_ref()),
    )
    .await
}

// Someone left the server. A linked account that was in it is flagged, and if it's a
// team member, their teammate and the hosts are told.
pub async fn left(state: &AppState, discord_id: &str) -> BotResult {
    let Some((user_id, was)) = db::discord::set_in_guild(&state.db, discord_id, false).await?
    else {
        return Ok(());
    };
    if was != Some(false) {
        on_left(state, user_id).await?;
    }
    Ok(())
}

async fn on_left(state: &AppState, user_id: Uuid) -> BotResult {
    db::discord::notify_left(&state.db, user_id).await?;
    state
        .discord
        .enqueue(&state.db, &[Job::MemberLeft { user_id }])
        .await?;
    Ok(())
}

async fn on_back(state: &AppState, user_id: Uuid, returned: bool) -> BotResult {
    let mut jobs: Vec<Job> = db::discord::open_matches_of(&state.db, user_id)
        .await?
        .into_iter()
        .map(|match_id| Job::SyncMatch { match_id })
        .collect();
    if returned {
        jobs.push(Job::MemberReturned { user_id });
    }
    state.discord.enqueue(&state.db, &jobs).await?;
    Ok(())
}
