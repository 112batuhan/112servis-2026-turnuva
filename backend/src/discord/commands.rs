//! Slash commands, registered to the tournament server whenever the bot connects, and
//! the buttons on the bot's messages.

use serenity::all::{
    CommandInteraction, CommandOptionType, ComponentInteraction, CreateAllowedMentions,
    CreateAutocompleteResponse, CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, EditInteractionResponse, Interaction, ResolvedValue,
};
use uuid::Uuid;

use super::{
    jobs::Embed,
    messages::{self, number, osu_profile, side_code, when, when_short},
    results, Bot,
};
use crate::{
    db::{
        self,
        bracket::Side,
        discord::{MatchInfo, TeamName},
        matches::MatchKind,
        notifications::Response,
    },
    handlers, team_rank, AppState,
};

const LIST_LIMIT: i64 = 10;

fn team_option() -> CreateCommandOption {
    CreateCommandOption::new(
        CommandOptionType::String,
        "takim",
        "Takım adı (boş bırakırsan kendi takımın)",
    )
    .set_autocomplete(true)
}

pub fn definitions() -> Vec<CreateCommand> {
    vec![
        CreateCommand::new("rank")
            .description("Oyuncu değerini, iki rank verirsen takım değerini hesaplar")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "rank1",
                    "Birinci oyuncunun global rankı",
                )
                .required(true)
                .min_int_value(1),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::Integer,
                    "rank2",
                    "İkinci oyuncunun global rankı",
                )
                .min_int_value(1),
            ),
        CreateCommand::new("maclar")
            .description("Yaklaşan maçlar")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "takim",
                    "Sadece bu takımın maçları",
                )
                .set_autocomplete(true),
            ),
        CreateCommand::new("takim")
            .description("Takım bilgisi")
            .add_option(team_option()),
        CreateCommand::new("gecmis")
            .description("Takımın maç geçmişi")
            .add_option(team_option()),
        CreateCommand::new("mac")
            .description("Maç detayları, bitmişse sonuçları")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "kod",
                    "Maç kodu, ör. W1 ya da L3",
                )
                .required(true)
                .set_autocomplete(true),
            ),
    ]
}

pub async fn handle(state: &AppState, bot: &Bot, interaction: Interaction) {
    let result = match interaction {
        Interaction::Command(cmd) => command(state, bot, &cmd).await,
        Interaction::Autocomplete(cmd) => autocomplete(state, bot, &cmd).await,
        Interaction::Component(comp) => component(state, bot, &comp).await,
        _ => Ok(()),
    };
    if let Err(err) = result {
        tracing::warn!(error = %err, "discord interaction failed");
    }
}

// ---------- commands ----------

enum Reply {
    Embed(Embed),
    Text(String),
}

async fn command(state: &AppState, bot: &Bot, cmd: &CommandInteraction) -> super::BotResult {
    // /mac reads osu! for a played match, which can take longer than Discord waits.
    let slow = cmd.data.name == "mac";
    if slow {
        cmd.defer(&bot.http).await?;
    }
    let reply = match run_command(state, cmd).await {
        Ok(reply) => reply,
        Err(err) => {
            tracing::error!(error = %err, command = %cmd.data.name, "discord command failed");
            Reply::Text("Bir hata oluştu, biraz sonra tekrar dene.".into())
        }
    };
    let ephemeral = matches!(reply, Reply::Text(_));
    if slow {
        let edit = match reply {
            Reply::Embed(e) => EditInteractionResponse::new().embed(e.build()),
            Reply::Text(t) => EditInteractionResponse::new().content(t),
        };
        cmd.edit_response(
            &bot.http,
            edit.allowed_mentions(CreateAllowedMentions::new()),
        )
        .await?;
    } else {
        let message = match reply {
            Reply::Embed(e) => CreateInteractionResponseMessage::new().embed(e.build()),
            Reply::Text(t) => CreateInteractionResponseMessage::new().content(t),
        };
        cmd.create_response(
            &bot.http,
            CreateInteractionResponse::Message(
                message
                    .ephemeral(ephemeral)
                    .allowed_mentions(CreateAllowedMentions::new()),
            ),
        )
        .await?;
    }
    Ok(())
}

fn int_option(cmd: &CommandInteraction, name: &str) -> Option<i64> {
    cmd.data.options().into_iter().find_map(|o| match o.value {
        ResolvedValue::Integer(n) if o.name == name => Some(n),
        _ => None,
    })
}

fn str_option(cmd: &CommandInteraction, name: &str) -> Option<String> {
    cmd.data.options().into_iter().find_map(|o| match o.value {
        ResolvedValue::String(s) if o.name == name => Some(s.trim().to_string()),
        _ => None,
    })
}

async fn run_command(state: &AppState, cmd: &CommandInteraction) -> super::BotResult<Reply> {
    match cmd.data.name.as_str() {
        "rank" => Ok(rank(
            int_option(cmd, "rank1").unwrap_or(1),
            int_option(cmd, "rank2"),
        )),
        "maclar" => {
            let team = match str_option(cmd, "takim") {
                Some(name) => match db::discord::find_team_by_name(&state.db, &name).await? {
                    Some(team) => Some(team),
                    None => return Ok(Reply::Text(format!("**{name}** adında bir takım yok."))),
                },
                None => None,
            };
            upcoming(state, team).await
        }
        "takim" | "gecmis" => {
            let team = match resolve_team(state, cmd).await? {
                Ok(team) => team,
                Err(text) => return Ok(Reply::Text(text)),
            };
            if cmd.data.name == "takim" {
                team_info(state, team).await
            } else {
                history(state, team).await
            }
        }
        "mac" => {
            let code = str_option(cmd, "kod").unwrap_or_default();
            let Some((side, n)) = parse_code(&code) else {
                return Ok(Reply::Text("Maç kodunu W1 ya da L3 gibi yaz.".into()));
            };
            let Some(id) = db::discord::bracket_match_by_code(&state.db, side, n).await? else {
                return Ok(Reply::Text(format!(
                    "{} diye bir maç yok.",
                    side_code(side, n)
                )));
            };
            match db::discord::match_info(&state.db, id).await? {
                Some(info) => match_details(state, &info).await,
                None => Ok(Reply::Text("Maç bulunamadı.".into())),
            }
        }
        _ => Ok(Reply::Text("Bilinmeyen komut.".into())),
    }
}

// The named team, or the caller's own.
async fn resolve_team(
    state: &AppState,
    cmd: &CommandInteraction,
) -> sqlx::Result<Result<TeamName, String>> {
    if let Some(name) = str_option(cmd, "takim") {
        return Ok(db::discord::find_team_by_name(&state.db, &name)
            .await?
            .ok_or(format!("**{name}** adında bir takım yok.")));
    }
    let Some(team_id) = db::discord::team_of_discord(&state.db, &cmd.user.id.to_string()).await?
    else {
        return Ok(Err("Bir takımda değilsin; takım adını yaz.".into()));
    };
    let players = db::discord::team_players(&state.db, team_id).await?;
    Ok(players
        .first()
        .map(|p| TeamName {
            id: team_id,
            name: p.team_name.clone(),
        })
        .ok_or_else(|| "Takım bulunamadı.".to_string()))
}

fn parse_code(code: &str) -> Option<(Side, i32)> {
    let code = code.trim();
    let side = match code.chars().next()?.to_ascii_uppercase() {
        'W' => Side::Winners,
        'L' => Side::Losers,
        _ => return None,
    };
    let n: i32 = code[1..].trim().parse().ok()?;
    (n > 0).then_some((side, n))
}

fn rank(rank1: i64, rank2: Option<i64>) -> Reply {
    let limit = team_rank::LIMIT;
    let value = |r: i64| number(team_rank::player_value(r).round() as i64);
    let mut embed = Embed {
        title: Some("Rank hesabı".into()),
        color: Some(messages::COLOR),
        ..Embed::default()
    };
    embed.fields.push((
        format!("#{}", number(rank1)),
        format!("Değer: **{}**", value(rank1)),
        true,
    ));
    match rank2 {
        Some(rank2) => {
            embed.fields.push((
                format!("#{}", number(rank2)),
                format!("Değer: **{}**", value(rank2)),
                true,
            ));
            let total = team_rank::team_value(rank1, rank2);
            let verdict = if total <= limit {
                "✅ Takım olabilirler"
            } else {
                "❌ Sınırın üzerinde"
            };
            embed.fields.push((
                "Takım değeri".into(),
                format!(
                    "**{}** / {}\n{verdict}",
                    number(total.round() as i64),
                    number(limit as i64)
                ),
                false,
            ));
        }
        None => {
            embed.fields.push((
                "En iyi partner".into(),
                format!(
                    "Takım arkadaşı en iyi **#{}** olabilir.",
                    number(team_rank::min_partner_rank(rank1))
                ),
                false,
            ));
        }
    }
    embed.footer = Some(format!("Takım değeri sınırı: {}", number(limit as i64)));
    Reply::Embed(embed)
}

// "`W1` Sokak Kedileri vs Tatlı Krizi · <time>", or a lobby with its team count.
fn match_line(info: &MatchInfo) -> String {
    let at = info
        .starts_at
        .map(|at| format!(" · {} (<t:{}:R>)", when_short(at), at.unix_timestamp()))
        .unwrap_or_default();
    match info.kind {
        MatchKind::Bracket => format!(
            "`{}` **{}** vs **{}**{at}",
            messages::code(info),
            info.team1_name.as_deref().unwrap_or("?"),
            info.team2_name.as_deref().unwrap_or("?"),
        ),
        MatchKind::Qualifier => format!("`{}` {} takım{at}", messages::code(info), info.entries),
    }
}

async fn upcoming(state: &AppState, team: Option<TeamName>) -> super::BotResult<Reply> {
    let ids =
        db::discord::upcoming_matches(&state.db, team.as_ref().map(|t| t.id), LIST_LIMIT).await?;
    let mut lines = Vec::new();
    for id in ids {
        if let Some(info) = db::discord::match_info(&state.db, id).await? {
            lines.push(match_line(&info));
        }
    }
    let title = match &team {
        Some(t) => format!("{} · yaklaşan maçlar", t.name),
        None => "Yaklaşan maçlar".into(),
    };
    Ok(Reply::Embed(Embed {
        title: Some(title),
        description: Some(if lines.is_empty() {
            "Planlanmış maç yok.".into()
        } else {
            lines.join("\n")
        }),
        url: Some(messages::site_url(state, "/bracket")),
        color: Some(messages::COLOR),
        ..Embed::default()
    }))
}

async fn team_info(state: &AppState, team: TeamName) -> super::BotResult<Reply> {
    let players = db::discord::team_players(&state.db, team.id).await?;
    let lines: Vec<String> = players
        .iter()
        .map(|p| {
            let rank = p
                .global_rank
                .map_or("rank yok".into(), |r| format!("#{}", number(r)));
            let discord = p
                .discord_id
                .as_deref()
                .filter(|id| super::snowflake::user(id).is_some())
                .map(|id| format!(" · <@{id}>"))
                .unwrap_or_default();
            format!("{} · {rank}{discord}", osu_profile(p.osu_id, &p.username))
        })
        .collect();
    let mut embed = Embed {
        title: Some(team.name.clone()),
        description: Some(lines.join("\n")),
        url: Some(messages::site_url(state, "/teams")),
        color: Some(messages::COLOR),
        ..Embed::default()
    };
    if let [Some(a), Some(b)] =
        [players.first(), players.get(1)].map(|p| p.and_then(|p| p.global_rank))
    {
        let value = team_rank::team_value(a, b);
        let mark = if value <= team_rank::LIMIT {
            ""
        } else {
            " ⚠️ sınırın üzerinde"
        };
        embed.fields.push((
            "Takım değeri".into(),
            format!(
                "{} / {}{mark}",
                number(value.round() as i64),
                number(team_rank::LIMIT as i64)
            ),
            true,
        ));
    }
    if let Some(id) = db::discord::upcoming_matches(&state.db, Some(team.id), 1)
        .await?
        .pop()
    {
        if let Some(info) = db::discord::match_info(&state.db, id).await? {
            embed
                .fields
                .push(("Sıradaki maç".into(), match_line(&info), false));
        }
    }
    if let Some(at) = db::discord::banner_version(&state.db, team.id).await? {
        embed.image = messages::banner_url(state, team.id, at);
    }
    Ok(Reply::Embed(embed))
}

async fn history(state: &AppState, team: TeamName) -> super::BotResult<Reply> {
    let mut lines = Vec::new();
    for id in db::discord::team_history(&state.db, team.id, 15).await? {
        let Some(info) = db::discord::match_info(&state.db, id).await? else {
            continue;
        };
        let first = info.team1_id == Some(team.id);
        let my_slot = if first { 1 } else { 2 };
        let (mine, theirs, opponent) = if first {
            (
                info.team1_score,
                info.team2_score,
                info.team2_name.as_deref(),
            )
        } else {
            (
                info.team2_score,
                info.team1_score,
                info.team1_name.as_deref(),
            )
        };
        let mark = if messages::winning_slot(&info) == Some(my_slot) {
            "✅"
        } else {
            "❌"
        };
        // A forfeit reads as W or FF instead of a score.
        let (mine, theirs) = match info.forfeit_slot {
            Some(slot) if slot == my_slot => ("FF".to_string(), "W".to_string()),
            Some(_) => ("W".to_string(), "FF".to_string()),
            None => (
                mine.unwrap_or(0).to_string(),
                theirs.unwrap_or(0).to_string(),
            ),
        };
        let at = info
            .concluded_at
            .map(|at| format!(" · <t:{}:d>", at.unix_timestamp()))
            .unwrap_or_default();
        lines.push(format!(
            "{mark} `{}` vs **{}** — **{mine} – {theirs}**{at}",
            messages::code(&info),
            opponent.unwrap_or("?")
        ));
    }
    Ok(Reply::Embed(Embed {
        title: Some(format!("{} · maç geçmişi", team.name)),
        description: Some(if lines.is_empty() {
            "Henüz oynanmış maç yok.".into()
        } else {
            lines.join("\n")
        }),
        color: Some(messages::COLOR),
        ..Embed::default()
    }))
}

async fn match_details(state: &AppState, info: &MatchInfo) -> super::BotResult<Reply> {
    if info.concluded_at.is_some() {
        return match results::results_embed(state, info).await {
            Ok(embed) => Ok(Reply::Embed(embed)),
            Err(err) => {
                tracing::warn!(error = %err, "could not build match results");
                Ok(Reply::Text(
                    "Maçın sonuçları şu an okunamadı, biraz sonra tekrar dene.".into(),
                ))
            }
        };
    }
    let players = db::discord::match_players(&state.db, info.id).await?;
    let mut embed = Embed {
        title: Some(messages::title(info)),
        url: Some(messages::site_url(
            state,
            &format!("/bracket?match={}", info.id),
        )),
        description: Some(match info.starts_at {
            Some(at) => format!("🕒 {}", when(at)),
            None => "🕒 Saati henüz belli değil.".into(),
        }),
        color: Some(messages::COLOR),
        ..Embed::default()
    };
    for team_id in [info.team1_id, info.team2_id].into_iter().flatten() {
        let members: Vec<&db::discord::MatchPlayer> =
            players.iter().filter(|p| p.team_id == team_id).collect();
        let Some(name) = members.first().map(|p| p.team_name.clone()) else {
            continue;
        };
        let names: Vec<String> = members
            .iter()
            .map(|p| osu_profile(p.osu_id, &p.username))
            .collect();
        embed.fields.push((name, names.join("\n"), true));
    }
    if let Some(stage) = &info.stage_name {
        embed.fields.push(("Aşama".into(), stage.clone(), false));
    }
    embed.fields.push((
        "Hakem".into(),
        info.referee_username
            .clone()
            .unwrap_or_else(|| "henüz yok".into()),
        true,
    ));
    embed.fields.push((
        "Yayıncı".into(),
        info.streamer_username
            .clone()
            .unwrap_or_else(|| "henüz yok".into()),
        true,
    ));
    if let Some(mp) = info.mp_ids.last() {
        embed.fields.push((
            "Oda".into(),
            format!("🎮 Oynanıyor: https://osu.ppy.sh/community/matches/{mp}"),
            false,
        ));
    }
    Ok(Reply::Embed(embed))
}

// ---------- autocomplete ----------

async fn autocomplete(state: &AppState, bot: &Bot, cmd: &CommandInteraction) -> super::BotResult {
    let Some(option) = cmd.data.autocomplete() else {
        return Ok(());
    };
    let query = option.value.trim();
    let mut response = CreateAutocompleteResponse::new();
    match option.name {
        "takim" => {
            for team in db::discord::search_teams(&state.db, query).await? {
                response = response.add_string_choice(team.name.clone(), team.name);
            }
        }
        "kod" => {
            let query = query.to_lowercase();
            let mut n = 0;
            for m in db::discord::match_codes(&state.db).await? {
                let code = side_code(m.side, m.number);
                let label = format!(
                    "{code} · {} vs {}",
                    m.team1_name.as_deref().unwrap_or("?"),
                    m.team2_name.as_deref().unwrap_or("?")
                );
                if !label.to_lowercase().contains(&query) {
                    continue;
                }
                response =
                    response.add_string_choice(label.chars().take(100).collect::<String>(), code);
                n += 1;
                if n == 25 {
                    break;
                }
            }
        }
        _ => {}
    }
    cmd.create_response(&bot.http, CreateInteractionResponse::Autocomplete(response))
        .await?;
    Ok(())
}

// ---------- buttons ----------

// "confirm:<id>" and "decline:<id>" answer a request to confirm a match's new time,
// like the buttons in the site's notifications.
async fn component(state: &AppState, bot: &Bot, comp: &ComponentInteraction) -> super::BotResult {
    let reply = |text: String| {
        CreateInteractionResponse::Message(
            CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(true),
        )
    };
    let Some((action, id)) = comp.data.custom_id.split_once(':') else {
        return Ok(());
    };
    let (Some(response), Ok(id)) = (
        match action {
            "confirm" => Some(Response::Confirmed),
            "decline" => Some(Response::Declined),
            _ => None,
        },
        id.parse::<Uuid>(),
    ) else {
        return Ok(());
    };
    let Some(account) = db::discord::find_linked(&state.db, &comp.user.id.to_string()).await?
    else {
        comp.create_response(
            &bot.http,
            reply("Bu Discord hesabı sitedeki bir hesaba bağlı değil.".into()),
        )
        .await?;
        return Ok(());
    };
    match handlers::notifications::answer_as(state, account.user_id, id, response).await {
        Ok(()) => {
            let note = match response {
                Response::Confirmed => "✅ Yeni saati onayladın.",
                Response::Declined => "❌ Katılamayacağını bildirdin, maçtan çıkarıldın.",
            };
            // The request is an embed (older ones were text), which an update without
            // embeds keeps; the answer goes above it.
            let content = if comp.message.content.is_empty() {
                note.to_string()
            } else {
                format!("{}\n\n{note}", comp.message.content)
            };
            comp.create_response(
                &bot.http,
                CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::new()
                        .content(content)
                        .components(Vec::new()),
                ),
            )
            .await?;
        }
        Err(err) => {
            comp.create_response(&bot.http, reply(format!("Olmadı: {err}")))
                .await?;
        }
    }
    Ok(())
}
