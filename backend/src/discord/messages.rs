//! What the bot says, in Turkish like the site. Each event turns into messages for the
//! places that should hear about it; jobs.rs queues one `Post` per message.

use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    jobs::{Button, Embed, Job, OutMessage, Target},
    BotResult,
};
use crate::{
    db::{
        self,
        bracket::Side,
        discord::{MatchInfo, MatchPlayer, Settings},
        matches::MatchKind,
        notifications::StaffRole,
    },
    team_rank, AppState,
};

// How many accounts one announcement message pings; Discord allows 100.
const MENTIONS_PER_MESSAGE: usize = 80;

pub const COLOR: u32 = 0x5865f2;
const COLOR_GOOD: u32 = 0x3ba55d;

// "W1", "L3" or "Lobi 2".
pub fn code(info: &MatchInfo) -> String {
    match (info.kind, info.side, info.number) {
        (MatchKind::Bracket, Some(side), Some(n)) => side_code(side, n),
        _ => format!("Lobi {}", info.lobby_number.unwrap_or(0)),
    }
}

pub fn side_code(side: Side, number: i32) -> String {
    match side {
        Side::Winners => format!("W{number}"),
        Side::Losers => format!("L{number}"),
    }
}

// "W1: Sokak Kedileri vs Tatlı Krizi" or "Lobi 2".
pub fn title(info: &MatchInfo) -> String {
    match info.kind {
        MatchKind::Bracket => format!(
            "{}: {} vs {}",
            code(info),
            info.team1_name.as_deref().unwrap_or("?"),
            info.team2_name.as_deref().unwrap_or("?"),
        ),
        MatchKind::Qualifier => format!("Eleme {}", code(info).to_lowercase()),
    }
}

// Shown in each reader's own time zone.
pub fn when(at: OffsetDateTime) -> String {
    let t = at.unix_timestamp();
    format!("<t:{t}:F> (<t:{t}:R>)")
}

pub fn when_short(at: OffsetDateTime) -> String {
    format!("<t:{}:f>", at.unix_timestamp())
}

// Thousands separated with dots, as in Turkish.
pub fn number(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 {
        out.insert(0, '-');
    }
    out
}

// "hakemi", "yayıncısı": the match's referee or streamer.
fn role_possessive(role: StaffRole) -> &'static str {
    match role {
        StaffRole::Referee => "hakemi",
        StaffRole::Streamer => "yayıncısı",
    }
}

fn role_name(role: StaffRole) -> &'static str {
    match role {
        StaffRole::Referee => "hakem",
        StaffRole::Streamer => "yayıncı",
    }
}

pub fn site_url(state: &AppState, path: &str) -> String {
    format!("{}{path}", state.config.frontend_url.trim_end_matches('/'))
}

pub fn osu_profile(osu_id: i64, username: &str) -> String {
    if osu_id > 0 {
        format!("[{username}](https://osu.ppy.sh/users/{osu_id})")
    } else {
        format!("**{username}**")
    }
}

// The people a message names, collected so they're also the ones it may ping.
#[derive(Default)]
pub struct Pings {
    pub ids: Vec<String>,
}

impl Pings {
    // A mention when the account is linked, otherwise the name in bold.
    pub fn person(&mut self, discord_id: Option<&str>, name: &str) -> String {
        match discord_id.filter(|id| super::snowflake::user(id).is_some()) {
            Some(id) => {
                if !self.ids.iter().any(|i| i == id) {
                    self.ids.push(id.to_string());
                }
                format!("<@{id}>")
            }
            None => format!("**{name}**"),
        }
    }

    fn player(&mut self, p: &MatchPlayer) -> String {
        self.person(p.discord_id.as_deref(), &p.username)
    }

    fn referee(&mut self, info: &MatchInfo) -> Option<String> {
        let name = info.referee_username.as_deref()?;
        Some(self.person(info.referee_discord_id.as_deref(), name))
    }

    fn streamer(&mut self, info: &MatchInfo) -> Option<String> {
        let name = info.streamer_username.as_deref()?;
        Some(self.person(info.streamer_discord_id.as_deref(), name))
    }

    // "**Team**: @a @b", one line per team.
    fn teams(&mut self, players: &[MatchPlayer]) -> String {
        let mut lines = Vec::new();
        for team in players.chunk_by(|a, b| a.team_id == b.team_id) {
            let names: Vec<String> = team.iter().map(|p| self.player(p)).collect();
            lines.push(format!("**{}**: {}", team[0].team_name, names.join(" ")));
        }
        lines.join("\n")
    }

    fn players_of(&mut self, players: &[MatchPlayer], team_id: Option<Uuid>) -> String {
        players
            .iter()
            .filter(|p| Some(p.team_id) == team_id)
            .map(|p| self.player(p))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

struct Ctx<'a> {
    state: &'a AppState,
    settings: Settings,
    out: Vec<(Target, OutMessage)>,
}

impl Ctx<'_> {
    fn post_match(&mut self, match_id: Uuid, content: String, pings: Pings) {
        self.out.push((
            Target::MatchChannel(match_id),
            OutMessage::text(content, pings.ids),
        ));
    }

    fn post_setting(&mut self, channel: Option<i64>, message: OutMessage) {
        if let Some(id) = channel {
            self.out.push((Target::Channel(id), message));
        }
    }

    fn post_hosts(&mut self, content: String) {
        // The overview names people without pinging them.
        self.post_setting(
            self.settings.host_channel_id,
            OutMessage::text(content, Vec::new()),
        );
    }

    fn post_user(&mut self, discord_id: Option<&str>, message: OutMessage) {
        if let Some(id) = discord_id {
            self.out.push((Target::User(id.to_string()), message));
        }
    }
}

async fn load_match(
    state: &AppState,
    match_id: Uuid,
) -> sqlx::Result<Option<(MatchInfo, Vec<MatchPlayer>)>> {
    let Some(info) = db::discord::match_info(&state.db, match_id).await? else {
        return Ok(None);
    };
    let players = db::discord::match_players(&state.db, match_id).await?;
    Ok(Some((info, players)))
}

async fn user(state: &AppState, id: Uuid) -> sqlx::Result<Option<db::discord::UserBrief>> {
    Ok(db::discord::users_brief(&state.db, &[id]).await?.pop())
}

pub async fn render(state: &AppState, job: &Job) -> BotResult<Vec<(Target, OutMessage)>> {
    let mut cx = Ctx {
        state,
        settings: db::discord::settings(&state.db).await?,
        out: Vec::new(),
    };
    match job {
        Job::TeamFormed { team_id } => team_formed(&mut cx, *team_id).await?,
        Job::TeamDisbanded {
            name,
            players,
            by_host,
        } => {
            let users = db::discord::users_brief(&state.db, players).await?;
            let names: Vec<String> = users
                .iter()
                .map(|u| format!("**{}**", u.username))
                .collect();
            let by = if *by_host { "host tarafından " } else { "" };
            cx.post_hosts(format!(
                "💔 **{name}** {by}dağıtıldı ({}).",
                names.join(", ")
            ));
        }
        Job::MemberLeft { user_id } => {
            let Some(u) = user(state, *user_id).await? else {
                return Ok(cx.out);
            };
            let discord = u.discord_username.as_deref().unwrap_or("?");
            let team = u
                .team_name
                .as_deref()
                .map(|t| format!(" Takımı: **{t}**."))
                .unwrap_or_default();
            cx.post_hosts(format!(
                "🚪 {} (Discord: `{discord}`) sunucudan ayrıldı.{team}",
                osu_profile(u.osu_id, &u.username)
            ));
            if let Some(team_id) = u.team_id {
                for mate in db::discord::team_players(&state.db, team_id).await? {
                    if mate.user_id == u.id {
                        continue;
                    }
                    cx.post_user(
                        mate.discord_id.as_deref(),
                        OutMessage::text(
                            format!(
                                "⚠️ Takım arkadaşın **{}** turnuva Discord sunucusundan ayrıldı. \
                                 Maç kanallarını görebilmesi için sunucuya geri dönmesi gerekiyor.",
                                u.username
                            ),
                            Vec::new(),
                        ),
                    );
                }
            }
        }
        Job::MemberReturned { user_id } => {
            if let Some(u) = user(state, *user_id).await? {
                cx.post_hosts(format!(
                    "↩️ {} sunucuya geri döndü.",
                    osu_profile(u.osu_id, &u.username)
                ));
            }
        }
        Job::RankOverLimit {
            team_id,
            user_id,
            old_rank,
            new_rank,
            team_value,
        } => {
            let u = user(state, *user_id).await?;
            let team = db::discord::team_players(&state.db, *team_id).await?;
            let Some(team_name) = team.first().map(|p| p.team_name.clone()) else {
                return Ok(cx.out);
            };
            let rank =
                |r: &Option<i64>| r.map_or("rank yok".to_string(), |r| format!("#{}", number(r)));
            cx.post_hosts(format!(
                "📈 **{team_name}** rank sınırını aştı: {} {} → {}. Takım değeri {} / {}.",
                u.map_or("?".to_string(), |u| format!("**{}**", u.username)),
                rank(old_rank),
                rank(new_rank),
                number(team_value.round() as i64),
                number(team_rank::LIMIT as i64),
            ));
        }
        _ => {
            if let Some(match_id) = match_id_of(job) {
                if let Some((info, players)) = load_match(state, match_id).await? {
                    match_event(&mut cx, job, &info, &players).await?;
                }
            }
        }
    }
    Ok(cx.out)
}

fn match_id_of(job: &Job) -> Option<Uuid> {
    match job {
        Job::MatchOpened { match_id }
        | Job::ProposalMade { match_id, .. }
        | Job::Rescheduled { match_id, .. }
        | Job::StaffAnswered { match_id, .. }
        | Job::StaffChanged { match_id, .. }
        | Job::TimeSet { match_id }
        | Job::RoomAdded { match_id, .. }
        | Job::Concluded { match_id }
        | Job::Reopened { match_id }
        | Job::Reminder { match_id }
        | Job::Announce { match_id }
        | Job::Ping { match_id, .. } => Some(*match_id),
        _ => None,
    }
}

// Mentions every player and the staff of a match.
fn everyone(pings: &mut Pings, info: &MatchInfo, players: &[MatchPlayer]) -> String {
    let mut names: Vec<String> = players.iter().map(|p| pings.player(p)).collect();
    names.extend(pings.referee(info));
    names.extend(pings.streamer(info));
    names.join(" ")
}

async fn match_event(
    cx: &mut Ctx<'_>,
    job: &Job,
    info: &MatchInfo,
    players: &[MatchPlayer],
) -> BotResult {
    let title = title(info);
    let id = info.id;
    match job {
        Job::MatchOpened { .. } => {
            let mut p = Pings::default();
            let mut lines = vec![format!("👋 **{title}** kanalına hoş geldiniz!")];
            if let Some(at) = info.starts_at {
                lines.push(format!("🕒 Maç saati: {}", when(at)));
            }
            if let Some(stage) = &info.stage_name {
                lines.push(format!("🗺️ Aşama: **{stage}**"));
            }
            lines.push(p.teams(players));
            lines.push(format!(
                "🧑‍⚖️ Hakem: {}",
                p.referee(info).unwrap_or_else(|| "henüz yok".into())
            ));
            if info.kind == MatchKind::Bracket {
                lines.push(format!(
                    "🎥 Yayıncı: {}",
                    p.streamer(info).unwrap_or_else(|| "henüz yok".into())
                ));
                lines.push(format!(
                    "Maçın saatini değiştirmek için takım sayfasından yeni saat önerebilirsiniz: {}",
                    site_url(cx.state, "/team")
                ));
            }
            lines.push("Maçla ilgili bildirimler bu kanala gelecek.".into());
            cx.post_match(id, lines.join("\n"), p);
        }
        Job::ProposalMade {
            team_id, starts_at, ..
        } => {
            let mut p = Pings::default();
            let proposer = players
                .iter()
                .find(|pl| pl.team_id == *team_id)
                .map_or("?".to_string(), |pl| pl.team_name.clone());
            let opponent = [info.team1_id, info.team2_id]
                .into_iter()
                .flatten()
                .find(|t| t != team_id);
            let mentions = p.players_of(players, opponent);
            cx.post_match(
                id,
                format!(
                    "📅 **{proposer}** yeni bir maç saati önerdi: {}\n{mentions} kabul etmek ya da başka bir saat önermek için: {}",
                    when(*starts_at),
                    site_url(cx.state, "/team")
                ),
                p,
            );
        }
        Job::Rescheduled {
            actor_id,
            old_starts_at,
            new_starts_at,
            ..
        } => {
            let actor = user(cx.state, *actor_id).await?;
            let actor = actor.map_or("?".to_string(), |u| u.username);
            let old = old_starts_at.map_or("belirsiz".to_string(), when_short);
            let mut p = Pings::default();
            let all = everyone(&mut p, info, players);
            cx.post_match(
                id,
                format!(
                    "✅ Maç saati değişti: ~~{old}~~ → **{}** (kabul eden: **{actor}**)\n{all}",
                    when(*new_starts_at)
                ),
                p,
            );
            for request in db::discord::confirm_requests(&cx.state.db, id).await? {
                let role = role_name(request.staff_role);
                cx.post_user(
                    Some(&request.discord_id),
                    OutMessage {
                        content: format!(
                            "🕒 **{title}** maçının saati değişti: {}\nBu saatte {role} olarak katılabilecek misin? \
                             Katılamazsan maçtan çıkarılırsın ve yerine biri aranır.",
                            when(*new_starts_at)
                        ),
                        buttons: vec![
                            Button {
                                custom_id: format!("confirm:{}", request.id),
                                label: "Katılabilirim".into(),
                                danger: false,
                            },
                            Button {
                                custom_id: format!("decline:{}", request.id),
                                label: "Katılamıyorum".into(),
                                danger: true,
                            },
                        ],
                        ..OutMessage::default()
                    },
                );
            }
            cx.post_hosts(format!(
                "📅 **{title}** taşındı: {old} → {} (kabul eden: **{actor}**)",
                when_short(*new_starts_at)
            ));
        }
        Job::StaffAnswered {
            user_id,
            role,
            declined,
            ..
        } => {
            let name = user(cx.state, *user_id)
                .await?
                .map_or("?".to_string(), |u| u.username);
            let role_text = role_name(*role);
            if !declined {
                cx.post_hosts(format!(
                    "👍 **{name}** ({role_text}) **{title}** maçının yeni saatini onayladı."
                ));
                return Ok(());
            }
            cx.post_hosts(format!(
                "❌ **{name}** ({role_text}) **{title}** maçının yeni saatine katılamıyor, maçtan çıkarıldı."
            ));
            let at = info.starts_at.map(when_short).unwrap_or_default();
            let channel = match role {
                StaffRole::Referee => cx.settings.referee_channel_id,
                StaffRole::Streamer => cx.settings.streamer_channel_id,
            };
            cx.post_setting(
                channel,
                OutMessage::text(
                    format!(
                        "🆘 **{title}** ({at}) için {role_text} aranıyor: **{name}** yeni saate katılamıyor. \
                         Almak için: {}",
                        site_url(cx.state, "/admin/bracket")
                    ),
                    Vec::new(),
                ),
            );
            cx.post_match(
                id,
                format!("ℹ️ **{name}** yeni saatte {role_text} olamayacak, yerine biri bulunacak."),
                Pings::default(),
            );
        }
        Job::StaffChanged {
            role, user_id, by, ..
        } => {
            let role_text = role_name(*role);
            match user_id {
                Some(user_id) => {
                    let Some(u) = user(cx.state, *user_id).await? else {
                        return Ok(());
                    };
                    let mut p = Pings::default();
                    let who = p.person(u.discord_id.as_deref(), &u.username);
                    let icon = if *role == StaffRole::Referee {
                        "🧑‍⚖️"
                    } else {
                        "🎥"
                    };
                    cx.post_match(
                        id,
                        format!("{icon} Bu maçın {}: {who}", role_possessive(*role)),
                        p,
                    );
                    if by != user_id {
                        let at = info
                            .starts_at
                            .map(|at| format!(" ({})", when_short(at)))
                            .unwrap_or_default();
                        cx.post_user(
                            u.discord_id.as_deref(),
                            OutMessage::text(
                                format!("📋 **{title}**{at} maçına {role_text} olarak atandın."),
                                Vec::new(),
                            ),
                        );
                    }
                }
                None => cx.post_match(
                    id,
                    format!("ℹ️ Bu maçın artık {} yok.", role_possessive(*role)),
                    Pings::default(),
                ),
            }
        }
        Job::TimeSet { .. } => {
            let mut p = Pings::default();
            let all = everyone(&mut p, info, players);
            let at = info.starts_at.map_or("belirsiz".to_string(), when);
            cx.post_match(id, format!("🕒 Maç saati güncellendi: **{at}**\n{all}"), p);
        }
        Job::RoomAdded { mp_id, .. } => {
            let mut p = Pings::default();
            let names: Vec<String> = players.iter().map(|pl| p.player(pl)).collect();
            let first = info.mp_ids.first() == Some(mp_id);
            let what = if first {
                "Maç odası açıldı"
            } else {
                "Yeni maç odası açıldı"
            };
            cx.post_match(
                id,
                format!(
                    "🎮 {what}: https://osu.ppy.sh/community/matches/{mp_id}\n{} oyun içi davetleri kontrol edin!",
                    names.join(" ")
                ),
                p,
            );
        }
        Job::Concluded { .. } => {
            let text = match (info.kind, info.team1_score, info.team2_score) {
                (MatchKind::Bracket, Some(s1), Some(s2)) => format!(
                    "🏁 Maç bitti: **{} {s1} – {s2} {}**. İyi oyundu! Kanal arşive taşındı.",
                    info.team1_name.as_deref().unwrap_or("?"),
                    info.team2_name.as_deref().unwrap_or("?"),
                ),
                _ => "🏁 Maç tamamlandı. Kanal arşive taşındı.".to_string(),
            };
            cx.post_match(id, text, Pings::default());
        }
        Job::Reopened { .. } => {
            cx.post_match(id, "↩️ Maç yeniden açıldı.".to_string(), Pings::default());
        }
        Job::Reminder { .. } => {
            let Some(at) = info.starts_at else {
                return Ok(());
            };
            let mut p = Pings::default();
            let all = everyone(&mut p, info, players);
            let mut text = format!(
                "⏰ **{title}** yaklaşık 1 saat sonra başlıyor ({})\n{all}",
                when(at)
            );
            if info.referee_id.is_none() {
                text.push_str("\n⚠️ Bu maçın henüz hakemi yok.");
                cx.post_setting(
                    cx.settings.referee_channel_id,
                    OutMessage::text(
                        format!("⚠️ **{title}** {} başlıyor ve hakemi yok!", when(at)),
                        Vec::new(),
                    ),
                );
            }
            if info.kind == MatchKind::Bracket && info.streamer_id.is_none() {
                cx.post_setting(
                    cx.settings.streamer_channel_id,
                    OutMessage::text(
                        format!("📺 **{title}** {} başlıyor ve yayıncısı yok.", when(at)),
                        Vec::new(),
                    ),
                );
            }
            cx.post_match(id, text, p);
        }
        Job::Announce { .. } => announce(cx, info, players).await?,
        Job::Ping { user_id, by, .. } => {
            let Some(player) = players.iter().find(|pl| pl.user_id == *user_id) else {
                return Ok(());
            };
            let by = user(cx.state, *by)
                .await?
                .map_or("Hakem".to_string(), |u| u.username);
            let mut p = Pings::default();
            let who = p.player(player);
            let room = info
                .mp_ids
                .last()
                .map(|mp| format!("\nOda: https://osu.ppy.sh/community/matches/{mp}"))
                .unwrap_or_default();
            cx.post_match(
                id,
                format!("🔔 {who}, hakem **{by}** seni maça çağırıyor!{room}"),
                p,
            );
        }
        _ => {}
    }
    Ok(())
}

// The public post before a bracket match starts, pinging those who asked for it.
async fn announce(cx: &mut Ctx<'_>, info: &MatchInfo, players: &[MatchPlayer]) -> BotResult {
    let Some(channel) = cx.settings.match_announce_channel_id else {
        return Ok(());
    };
    let Some(at) = info.starts_at else {
        return Ok(());
    };
    // Names only: the players and staff hear about it in their own channel.
    let mut silent = Pings::default();
    let mut embed = Embed {
        title: Some(title(info)),
        url: Some(site_url(cx.state, &format!("/bracket?match={}", info.id))),
        description: Some(format!("Maç {} başlıyor!", when(at))),
        color: Some(COLOR),
        ..Embed::default()
    };
    for team_id in [info.team1_id, info.team2_id].into_iter().flatten() {
        let Some(name) = players
            .iter()
            .find(|p| p.team_id == team_id)
            .map(|p| p.team_name.clone())
        else {
            continue;
        };
        embed
            .fields
            .push((name, silent.players_of(players, Some(team_id)), true));
    }
    if let Some(stage) = &info.stage_name {
        embed.fields.push(("Aşama".into(), stage.clone(), false));
    }
    if let Some(r) = silent.referee(info) {
        embed.fields.push(("Hakem".into(), r, true));
    }
    if let Some(s) = silent.streamer(info) {
        embed.fields.push(("Yayıncı".into(), s, true));
    }

    let subscribers = db::discord::subscriber_discord_ids(&cx.state.db, info.id).await?;
    let mut chunks = subscribers.chunks(MENTIONS_PER_MESSAGE);
    let first: Vec<String> = chunks.next().map(<[String]>::to_vec).unwrap_or_default();
    let mentions = first
        .iter()
        .map(|id| format!("<@{id}>"))
        .collect::<Vec<_>>()
        .join(" ");
    cx.out.push((
        super::jobs::Target::Channel(channel),
        OutMessage {
            content: format!("📣 **{}** birazdan başlıyor! {mentions}", title(info)),
            mentions: first,
            embed: Some(embed),
            buttons: Vec::new(),
        },
    ));
    for chunk in chunks {
        let mentions = chunk
            .iter()
            .map(|id| format!("<@{id}>"))
            .collect::<Vec<_>>()
            .join(" ");
        cx.out.push((
            super::jobs::Target::Channel(channel),
            OutMessage::text(mentions, chunk.to_vec()),
        ));
    }
    Ok(())
}

// A new team, in the announcement channel unless both players were announced within the
// week, and in the hosts' overview.
async fn team_formed(cx: &mut Ctx<'_>, team_id: Uuid) -> BotResult {
    let players = db::discord::team_players(&cx.state.db, team_id).await?;
    let Some(name) = players.first().map(|p| p.team_name.clone()) else {
        return Ok(());
    };
    let names: Vec<String> = players
        .iter()
        .map(|p| format!("**{}**", p.username))
        .collect();
    cx.post_hosts(format!("🤝 Yeni takım: **{name}** ({})", names.join(", ")));

    let Some(channel) = cx.settings.team_announce_channel_id else {
        return Ok(());
    };
    let ids: Vec<Uuid> = players.iter().map(|p| p.user_id).collect();
    let mut tx = cx.state.db.begin().await?;
    let announce = db::discord::take_team_announcement(&mut tx, &ids).await?;
    tx.commit().await?;
    if !announce {
        return Ok(());
    }

    let mut p = Pings::default();
    let lines: Vec<String> = players
        .iter()
        .map(|pl| {
            let rank = pl
                .global_rank
                .map_or("rank yok".into(), |r| format!("#{}", number(r)));
            format!(
                "{} · {rank} · {}",
                osu_profile(pl.osu_id, &pl.username),
                p.player(pl)
            )
        })
        .collect();
    let mut embed = Embed {
        title: Some(format!("🎉 Yeni takım: {name}")),
        description: Some(lines.join("\n")),
        url: Some(site_url(cx.state, "/teams")),
        color: Some(COLOR_GOOD),
        ..Embed::default()
    };
    if let [Some(a), Some(b)] =
        [players.first(), players.get(1)].map(|p| p.and_then(|p| p.global_rank))
    {
        embed.fields.push((
            "Takım değeri".into(),
            format!(
                "{} / {}",
                number(team_rank::team_value(a, b).round() as i64),
                number(team_rank::LIMIT as i64)
            ),
            true,
        ));
    }
    cx.out.push((
        super::jobs::Target::Channel(channel),
        OutMessage {
            content: String::new(),
            mentions: p.ids,
            embed: Some(embed),
            buttons: Vec::new(),
        },
    ));
    Ok(())
}
