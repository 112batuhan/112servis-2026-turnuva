//! A concluded bracket match's results: posted in the results channel, and deleted
//! again if the match is reopened. /mac shows the same.

use std::collections::HashMap;

use serenity::all::MessageId;
use uuid::Uuid;

use super::{
    jobs::Embed,
    messages::{self, number, side_code, site_url},
    snowflake, Bot, BotError, BotResult,
};
use crate::{
    db::{self, discord::MatchInfo, matches::MatchKind},
    osu_api, AppState,
};

pub async fn post(state: &AppState, bot: &Bot, match_id: Uuid) -> BotResult {
    let settings = db::discord::settings(&state.db).await?;
    let Some(channel) = settings.results_channel_id else {
        return Ok(());
    };
    let Some(info) = db::discord::match_info(&state.db, match_id).await? else {
        return Ok(());
    };
    // Reopened before this ran.
    if info.kind != MatchKind::Bracket || info.concluded_at.is_none() {
        return Ok(());
    }
    if db::discord::match_discord(&state.db, match_id)
        .await?
        .results_message_id
        .is_some()
    {
        return Ok(());
    }
    let embed = results_embed(state, &info).await?;
    let message = snowflake::channel(channel)
        .send_message(
            &bot.http,
            super::jobs::OutMessage {
                embed: Some(embed),
                ..Default::default()
            }
            .build(),
        )
        .await?;
    db::discord::set_results(
        &state.db,
        match_id,
        Some((channel, message.id.get() as i64)),
    )
    .await?;
    Ok(())
}

pub async fn delete(state: &AppState, bot: &Bot, match_id: Uuid) -> BotResult {
    let stored = db::discord::match_discord(&state.db, match_id).await?;
    if let (Some(channel), Some(message)) = (stored.results_channel_id, stored.results_message_id) {
        let deleted = snowflake::channel(channel)
            .delete_message(&bot.http, MessageId::new(message as u64))
            .await
            .map_err(BotError::from);
        match deleted {
            Ok(()) => {}
            Err(err) if err.not_found() => {}
            Err(err) => return Err(err),
        }
    }
    db::discord::set_results(&state.db, match_id, None).await?;
    Ok(())
}

// The score, the bans, and each pool map played with who picked it and both teams'
// scores, from the match's osu! rooms. Maps outside the pool (warmups) are left out.
pub async fn results_embed(state: &AppState, info: &MatchInfo) -> BotResult<Embed> {
    let team1 = info.team1_name.as_deref().unwrap_or("?");
    let team2 = info.team2_name.as_deref().unwrap_or("?");
    let (s1, s2) = (info.team1_score.unwrap_or(0), info.team2_score.unwrap_or(0));
    let code = match (info.side, info.number) {
        (Some(side), Some(n)) => side_code(side, n),
        _ => messages::code(info),
    };

    let players = db::discord::match_players(&state.db, info.id).await?;
    let team_of: HashMap<i64, Uuid> = players.iter().map(|p| (p.osu_id, p.team_id)).collect();
    let name_of = |team: Uuid| {
        if Some(team) == info.team1_id {
            team1
        } else if Some(team) == info.team2_id {
            team2
        } else {
            "?"
        }
    };

    let maps = match info.stage_id {
        Some(stage) => db::discord::stage_maps(&state.db, stage).await?,
        None => Vec::new(),
    };
    let label_of = |map_id: Uuid| {
        maps.iter()
            .find(|m| m.map_id == map_id)
            .map(|m| m.label.as_str())
    };
    let bans = db::bracket::list_bans(&state.db, info.id).await?;
    let picks = db::bracket::list_picks(&state.db, info.id).await?;

    let mut lines = Vec::new();
    let token = osu_api::token::app_token(&state.osu_client, &state.osu_app_token).await?;
    for &mp_id in &info.mp_ids {
        for game in osu_api::matches::fetch_games(&state.http_client, &token, mp_id).await? {
            let Some(map) = game
                .beatmap_id
                .and_then(|b| maps.iter().find(|m| m.beatmap_id == b))
            else {
                continue;
            };
            let total = |team: Option<Uuid>| -> i64 {
                game.scores
                    .iter()
                    .filter(|s| team.is_some() && team_of.get(&s.user_id).copied() == team)
                    .map(|s| s.score)
                    .sum()
            };
            let (t1, t2) = (total(info.team1_id), total(info.team2_id));
            let winner = match t1.cmp(&t2) {
                std::cmp::Ordering::Greater => team1,
                std::cmp::Ordering::Less => team2,
                std::cmp::Ordering::Equal => "berabere",
            };
            let picker = picks
                .iter()
                .find(|p| p.map_id == map.map_id)
                .map(|p| format!(" · seçen: {}", name_of(p.team_id)))
                .unwrap_or_default();
            lines.push(format!(
                "`{}` [{} [{}]](https://osu.ppy.sh/b/{}){picker}\n└ **{winner}** · {} – {}",
                map.label,
                map.title,
                map.version,
                map.beatmap_id,
                number(t1),
                number(t2),
            ));
        }
    }

    let mut embed = Embed {
        title: Some(format!("{code}: {team1} {s1} – {s2} {team2}")),
        url: Some(site_url(state, &format!("/bracket?match={}", info.id))),
        color: Some(messages::COLOR),
        ..Embed::default()
    };
    let mut description = String::new();
    for line in lines {
        // An embed's description holds at most 4096 characters.
        if description.chars().count() + line.chars().count() + 1 > 4000 {
            description.push_str("\n…");
            break;
        }
        if !description.is_empty() {
            description.push('\n');
        }
        description.push_str(&line);
    }
    if !description.is_empty() {
        embed.description = Some(description);
    }
    let winner = match s1.cmp(&s2) {
        std::cmp::Ordering::Greater => Some(team1),
        std::cmp::Ordering::Less => Some(team2),
        std::cmp::Ordering::Equal => None,
    };
    if let Some(w) = winner {
        embed
            .fields
            .push(("Kazanan".into(), format!("🏆 **{w}**"), true));
    }
    if let Some(stage) = &info.stage_name {
        embed.fields.push(("Aşama".into(), stage.clone(), true));
    }
    let mut ban_lines = Vec::new();
    for team in [info.team1_id, info.team2_id].into_iter().flatten() {
        let labels: Vec<&str> = bans
            .iter()
            .filter(|b| b.team_id == team)
            .filter_map(|b| label_of(b.map_id))
            .collect();
        if !labels.is_empty() {
            ban_lines.push(format!("{}: {}", name_of(team), labels.join(", ")));
        }
    }
    if !ban_lines.is_empty() {
        embed
            .fields
            .push(("Banlar".into(), ban_lines.join("\n"), false));
    }
    if let Some(referee) = &info.referee_username {
        embed.fields.push(("Hakem".into(), referee.clone(), true));
    }
    if let Some(streamer) = &info.streamer_username {
        embed
            .fields
            .push(("Yayıncı".into(), streamer.clone(), true));
    }
    if !info.mp_ids.is_empty() {
        let rooms: Vec<String> = info
            .mp_ids
            .iter()
            .map(|id| format!("[#{id}](https://osu.ppy.sh/community/matches/{id})"))
            .collect();
        embed.fields.push(("Odalar".into(), rooms.join(" "), false));
    }
    Ok(embed)
}
