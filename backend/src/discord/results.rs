//! A concluded bracket match's results: posted in the results channel, and deleted
//! again if the match is reopened. /mac shows the same.

use uuid::Uuid;

use super::{
    channels,
    jobs::Embed,
    messages::{self, match_page_url, number, side_code},
    snowflake, Bot, BotResult,
};
use crate::{
    db::{self, discord::MatchInfo, matches::MatchKind},
    AppState,
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
    // A post still on record (from before a reopening whose DeleteResults didn't get
    // through, or a retry of this job) is replaced, so there's one post per match.
    let stored = db::discord::match_discord(&state.db, match_id).await?;
    if let (Some(old_channel), Some(old_message)) =
        (stored.results_channel_id, stored.results_message_id)
    {
        channels::delete_message(bot, old_channel, old_message).await?;
        db::discord::set_results(&state.db, match_id, None).await?;
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
        channels::delete_message(bot, channel, message).await?;
    }
    db::discord::set_results(&state.db, match_id, None).await?;
    Ok(())
}

// The score, the bans, and each pool map played with who picked it and both teams'
// scores, all from the site: the map results the referee page saved with the score
// (read from the room's chat or synced from osu!) and the maps the referee awarded by
// hand. The rooms are linked, but osu! is never asked about them, so a room it has no
// record of can't hold the post back.
pub async fn results_embed(state: &AppState, info: &MatchInfo) -> BotResult<Embed> {
    let team1 = info.team1_name.as_deref().unwrap_or("?");
    let team2 = info.team2_name.as_deref().unwrap_or("?");

    let code = match (info.side, info.number) {
        (Some(side), Some(n)) => side_code(side, n),
        _ => messages::code(info),
    };

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
    let map_of = |map_id: Uuid| maps.iter().find(|m| m.map_id == map_id);
    let label_of = |map_id: Uuid| map_of(map_id).map(|m| m.label.as_str());
    let bans = db::bracket::list_bans(&state.db, info.id).await?;
    let picks = db::bracket::list_picks(&state.db, info.id).await?;
    let manual = db::bracket::list_manual_results(&state.db, info.id).await?;
    let played = db::bracket::list_map_results(&state.db, info.id).await?;
    let players = db::discord::match_players(&state.db, info.id).await?;

    let mut lines = Vec::new();
    for result in &played {
        let (t1, t2) = (result.team1_score, result.team2_score);
        let winner = match t1.cmp(&t2) {
            std::cmp::Ordering::Greater => team1,
            std::cmp::Ordering::Less => team2,
            std::cmp::Ordering::Equal => "berabere",
        };
        let picker = picks
            .iter()
            .find(|p| Some(p.map_id) == result.map_id)
            .map(|p| format!(" · seçen: {}", name_of(p.team_id)))
            .unwrap_or_default();
        let map = result.map_id.and_then(map_of);
        let what = match map {
            Some(map) => format!(
                "`{}` [{} [{}]](https://osu.ppy.sh/b/{})",
                map.label, map.title, map.version, map.beatmap_id
            ),
            // Deleted from the site, or the stage's pool changed since.
            None => "Havuzdan çıkmış map".to_string(),
        };
        let mut line = format!(
            "{what}{picker}\n└ **{winner}** · {} – {}",
            number(t1),
            number(t2),
        );
        // On freemod and tiebreaker maps the players chose their mods: each one's, by team.
        if map.is_some_and(|m| is_freemod(&m.label)) {
            let sides = [(info.team1_id, "🔴"), (info.team2_id, "🔵")]
                .into_iter()
                .filter_map(|(team, mark)| {
                    let scores: Vec<String> = result
                        .players
                        .iter()
                        .filter_map(|s| {
                            let p = players.iter().find(|p| p.user_id == s.user_id)?;
                            (Some(p.team_id) == team).then(|| {
                                format!("{} `{}` {}", p.username, mods_label(s), number(s.score))
                            })
                        })
                        .collect();
                    (!scores.is_empty()).then(|| format!("{mark} {}", scores.join(" · ")))
                })
                .collect::<Vec<_>>();
            if !sides.is_empty() {
                line.push_str(&format!("\n└ {}", sides.join("  ")));
            }
        }
        lines.push(line);
    }

    // Map wins the referee gave by hand, after the ones the rooms settled.
    for result in &manual {
        let what = match result.map_id.and_then(map_of) {
            Some(map) => format!(
                "`{}` [{} [{}]](https://osu.ppy.sh/b/{})",
                map.label, map.title, map.version, map.beatmap_id
            ),
            None => "Map dışı puan".to_string(),
        };
        let note = if result.note.is_empty() {
            String::new()
        } else {
            format!(" · {}", result.note)
        };
        lines.push(format!(
            "{what}\n└ **{}** · hakem kararı{note}",
            name_of(result.team_id)
        ));
    }

    let score = messages::result_line(info).unwrap_or_else(|| format!("{team1} – {team2}"));
    let mut embed = Embed {
        title: Some(format!("{code}: {score}")),
        url: Some(match_page_url(state, info.id)),
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
    let winner = match messages::winning_slot(info) {
        Some(1) => Some(team1),
        Some(_) => Some(team2),
        None => None,
    };
    if let Some(w) = winner {
        let how = if info.forfeit_slot.is_some() {
            " (hükmen)"
        } else {
            ""
        };
        embed
            .fields
            .push(("Kazanan".into(), format!("🏆 **{w}**{how}"), true));
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
    embed.fields.push((
        "Maç sayfası".into(),
        format!(
            "[Map sonuçları ve oda sohbeti]({})",
            match_page_url(state, info.id)
        ),
        false,
    ));
    Ok(embed)
}

// A freemod (FM1) or tiebreaker (TB) map, by its pool label, as the referee page tells them.
fn is_freemod(label: &str) -> bool {
    let label = label.trim().to_ascii_uppercase();
    label.starts_with("FM") || label.starts_with("TB")
}

// A player's mods as acronyms ("HDHR"), "NM" for none, "?" when they weren't read.
fn mods_label(score: &db::bracket::MapPlayerScore) -> String {
    match score.mods.as_deref() {
        None => "?".to_string(),
        Some("") => "NM".to_string(),
        Some(mods) => mods.to_ascii_uppercase(),
    }
}
