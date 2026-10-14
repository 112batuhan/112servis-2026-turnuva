//! What the bot says, in Turkish like the site. Each event turns into messages for the
//! places that should hear about it; jobs.rs queues one `Post` per message. What happens
//! to a match goes to its channel as plain text pinging its players and staff; posts
//! for the hosts and staff channels and DMs are embeds (a mention inside an embed never
//! pings, so anyone an embed should ping is mentioned above it). Each match channel also
//! has a pinned card (`match_card`) that channels.rs edits as the match changes.

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
const COLOR_WARN: u32 = 0xfaa61a;
const COLOR_BAD: u32 = 0xed4245;
// A finished match.
const COLOR_DONE: u32 = 0x747f8d;

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

// The match on the public site.
fn match_url(state: &AppState, info: &MatchInfo) -> String {
    match info.kind {
        MatchKind::Bracket => site_url(state, &format!("/bracket?match={}", info.id)),
        MatchKind::Qualifier => site_url(state, "/qualifiers"),
    }
}

// Where staff take the match.
fn admin_url(state: &AppState, info: &MatchInfo) -> String {
    match info.kind {
        MatchKind::Bracket => site_url(state, "/admin/bracket"),
        MatchKind::Qualifier => site_url(state, "/admin/qualifiers"),
    }
}

fn room_url(mp_id: i64) -> String {
    format!("https://osu.ppy.sh/community/matches/{mp_id}")
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

    fn players_of(&mut self, players: &[MatchPlayer], team_id: Option<Uuid>) -> String {
        players
            .iter()
            .filter(|p| Some(p.team_id) == team_id)
            .map(|p| self.player(p))
            .collect::<Vec<_>>()
            .join(" ")
    }

    // Mentions every player and the staff of a match.
    fn everyone(&mut self, info: &MatchInfo, players: &[MatchPlayer]) -> String {
        let mut names: Vec<String> = players.iter().map(|p| self.player(p)).collect();
        names.extend(self.referee(info));
        names.extend(self.streamer(info));
        names.join(" ")
    }
}

struct Ctx<'a> {
    state: &'a AppState,
    settings: Settings,
    out: Vec<(Target, OutMessage)>,
}

impl Ctx<'_> {
    // Plain text with the mentions in it, pinging the people it names.
    fn post_match(&mut self, match_id: Uuid, content: String, pings: Pings) {
        self.out.push((
            Target::MatchChannel(match_id),
            OutMessage::text(content, pings.ids),
        ));
    }

    // Names people without pinging them.
    fn post_setting(&mut self, channel: Option<i64>, embed: Embed) {
        if let Some(id) = channel {
            self.out
                .push((Target::Channel(id), OutMessage::embed(embed, Vec::new())));
        }
    }

    // An embed that pings `ids`, with any over Discord's limit for one message in
    // mention-only messages after it.
    fn post_setting_pinging(&mut self, channel: Option<i64>, embed: Embed, ids: &[String]) {
        let Some(id) = channel else {
            return;
        };
        let mut chunks = ids.chunks(MENTIONS_PER_MESSAGE);
        let first = chunks.next().map(<[String]>::to_vec).unwrap_or_default();
        self.out
            .push((Target::Channel(id), OutMessage::embed(embed, first)));
        for chunk in chunks {
            let mentions = chunk
                .iter()
                .map(|id| format!("<@{id}>"))
                .collect::<Vec<_>>()
                .join(" ");
            self.out.push((
                Target::Channel(id),
                OutMessage::text(mentions, chunk.to_vec()),
            ));
        }
    }

    fn post_hosts(&mut self, embed: Embed) {
        self.post_setting(self.settings.host_channel_id, embed);
    }

    fn post_user(&mut self, discord_id: Option<&str>, message: OutMessage) {
        if let Some(id) = discord_id {
            self.out.push((Target::User(id.to_string()), message));
        }
    }
}

// The pinned card at the top of a match's channel: the match as it is now. channels.rs
// posts it with the channel (pinging everyone in the match) and edits it whenever this
// renders differently, so it has no "last updated" time.
pub fn match_card(state: &AppState, info: &MatchInfo, players: &[MatchPlayer]) -> OutMessage {
    let mut p = Pings::default();
    let concluded = info.concluded_at.is_some();
    let mut about = vec![
        "Maçla ilgili bildirimler bu kanala gelir; bu mesaj maç değiştikçe güncellenir."
            .to_string(),
    ];
    if info.kind == MatchKind::Bracket && !concluded {
        about.push(format!(
            "Maçın saatini değiştirmek için takım sayfasından yeni saat önerebilirsiniz: {}",
            site_url(state, "/team")
        ));
    }
    let mut embed = Embed::new(
        format!("📌 {}", title(info)),
        if concluded { COLOR_DONE } else { COLOR },
    )
    .url(match_url(state, info))
    .description(about.join("\n\n"))
    .field(
        "🕒 Maç saati",
        info.starts_at
            .map_or("Henüz belirlenmedi".to_string(), when),
        false,
    );
    if let Some(stage) = &info.stage_name {
        embed = embed.field("🗺️ Aşama", stage, false);
    }

    for team in players.chunk_by(|a, b| a.team_id == b.team_id) {
        // Bracket teams in their colours on the referee page: team 1 red, team 2 blue.
        let side = if info.kind != MatchKind::Bracket {
            ""
        } else if Some(team[0].team_id) == info.team1_id {
            "🔴 "
        } else {
            "🔵 "
        };
        let names: Vec<String> = team.iter().map(|pl| p.player(pl)).collect();
        embed = embed.field(
            format!("{side}{}", team[0].team_name),
            names.join("\n"),
            true,
        );
    }
    if players.is_empty() {
        embed = embed.field("👥 Takımlar", "Henüz takım yok", false);
    }

    embed = embed.field(
        "🧑‍⚖️ Hakem",
        p.referee(info).unwrap_or_else(|| "Henüz yok".into()),
        true,
    );
    if info.kind == MatchKind::Bracket {
        embed = embed.field(
            "🎥 Yayıncı",
            p.streamer(info).unwrap_or_else(|| "Henüz yok".into()),
            true,
        );
    }
    // The latest room is the one being played; older ones broke.
    if let Some((latest, older)) = info.mp_ids.split_last() {
        let mut rooms = format!("[Oda #{latest}]({})", room_url(*latest));
        if !older.is_empty() {
            let old: Vec<String> = older
                .iter()
                .map(|id| format!("[#{id}]({})", room_url(*id)))
                .collect();
            rooms.push_str(&format!("\nÖnceki: {}", old.join(", ")));
        }
        embed = embed.field("🎮 Maç odası", rooms, false);
    }
    if concluded {
        let result = match (info.kind, info.team1_score, info.team2_score) {
            (MatchKind::Bracket, Some(s1), Some(s2)) => format!(
                "**{} {s1} – {s2} {}**",
                info.team1_name.as_deref().unwrap_or("?"),
                info.team2_name.as_deref().unwrap_or("?"),
            ),
            _ => "Maç tamamlandı.".to_string(),
        };
        embed = embed.field("🏁 Sonuç", result, false);
    }
    OutMessage::embed(embed, p.ids)
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
                .map(|u| osu_profile(u.osu_id, &u.username))
                .collect();
            let mut text = format!("Oyuncular: {}", names.join(", "));
            if *by_host {
                text.push_str("\nHost tarafından dağıtıldı.");
            }
            cx.post_hosts(
                Embed::new(format!("💔 Takım dağıtıldı: {name}"), COLOR_BAD).description(text),
            );
        }
        Job::MemberLeft { user_id } => {
            let Some(u) = user(state, *user_id).await? else {
                return Ok(cx.out);
            };
            let mut embed = Embed::new("🚪 Sunucudan ayrıldı", COLOR_WARN)
                .description(osu_profile(u.osu_id, &u.username))
                .field(
                    "Discord",
                    format!("`{}`", u.discord_username.as_deref().unwrap_or("?")),
                    true,
                );
            if let Some(team) = &u.team_name {
                embed = embed.field("Takım", team, true);
            }
            cx.post_hosts(embed);
            if let Some(team_id) = u.team_id {
                for mate in db::discord::team_players(&state.db, team_id).await? {
                    if mate.user_id == u.id {
                        continue;
                    }
                    cx.post_user(
                        mate.discord_id.as_deref(),
                        OutMessage::embed(
                            Embed::new("⚠️ Takım arkadaşın sunucudan ayrıldı", COLOR_WARN)
                                .description(format!(
                                    "**{}** turnuva Discord sunucusundan ayrıldı. Maç kanallarını \
                                     görebilmesi için sunucuya geri dönmesi gerekiyor.",
                                    u.username
                                )),
                            Vec::new(),
                        ),
                    );
                }
            }
        }
        Job::MemberReturned { user_id } => {
            if let Some(u) = user(state, *user_id).await? {
                cx.post_hosts(
                    Embed::new("↩️ Sunucuya geri döndü", COLOR_GOOD)
                        .description(osu_profile(u.osu_id, &u.username)),
                );
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
            cx.post_hosts(
                Embed::new(format!("📈 Rank sınırı aşıldı: {team_name}"), COLOR_WARN)
                    .url(site_url(state, "/admin/teams"))
                    .field(
                        "Oyuncu",
                        u.map_or("?".to_string(), |u| osu_profile(u.osu_id, &u.username)),
                        true,
                    )
                    .field(
                        "Rank",
                        format!("{} → {}", rank(old_rank), rank(new_rank)),
                        true,
                    )
                    .field(
                        "Takım değeri",
                        format!(
                            "{} / {}",
                            number(team_value.round() as i64),
                            number(team_rank::LIMIT as i64),
                        ),
                        true,
                    ),
            );
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
        Job::ProposalMade { match_id, .. }
        | Job::Rescheduled { match_id, .. }
        | Job::StaffAnswered { match_id, .. }
        | Job::StaffChanged { match_id, .. }
        | Job::TimeSet { match_id }
        | Job::ConfirmTime { match_id }
        | Job::RoomAdded { match_id, .. }
        | Job::Concluded { match_id }
        | Job::Reopened { match_id }
        | Job::Reminder { match_id }
        | Job::Announce { match_id }
        | Job::Ping { match_id, .. } => Some(*match_id),
        _ => None,
    }
}

// What happens to a match is posted in its channel as plain text that pings everyone it
// concerns, its players, referee and streamer, so nobody misses a change; the pinned
// card (edited by the SyncMatch queued with each change) shows the result. Only the
// posts for the staff and hosts elsewhere, and the DMs, are embeds.
async fn match_event(
    cx: &mut Ctx<'_>,
    job: &Job,
    info: &MatchInfo,
    players: &[MatchPlayer],
) -> BotResult {
    let title = title(info);
    let id = info.id;
    match job {
        Job::ProposalMade {
            team_id, starts_at, ..
        } => {
            // Only the other team can accept it.
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
                    "📅 **{proposer}** yeni bir maç saati önerdi: {}\n{mentions} kabul etmek ya da \
                     başka bir saat önermek için: <{}>",
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
            let all = p.everyone(info, players);
            cx.post_match(
                id,
                format!(
                    "✅ Maç saati değişti: ~~{old}~~ → **{}** (kabul eden: **{actor}**)\n{all}",
                    when(*new_starts_at)
                ),
                p,
            );
            ask_staff(cx, info, &title, *new_starts_at).await?;
            cx.post_hosts(
                Embed::new(format!("📅 Maç taşındı: {title}"), COLOR)
                    .url(match_url(cx.state, info))
                    .field("Eski saat", old, true)
                    .field("Yeni saat", when_short(*new_starts_at), true)
                    .field("Kabul eden", format!("**{actor}**"), true),
            );
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
                cx.post_hosts(
                    Embed::new("👍 Yeni saat onaylandı", COLOR_GOOD).description(format!(
                        "**{name}** ({role_text}) **{title}** maçının yeni saatini onayladı."
                    )),
                );
                return Ok(());
            }
            cx.post_hosts(
                Embed::new("❌ Yeni saate katılamıyor", COLOR_BAD).description(format!(
                    "**{name}** ({role_text}) **{title}** maçının yeni saatine katılamıyor, \
                     maçtan çıkarıldı."
                )),
            );
            let channel = match role {
                StaffRole::Referee => cx.settings.referee_channel_id,
                StaffRole::Streamer => cx.settings.streamer_channel_id,
            };
            let admin = admin_url(cx.state, info);
            let mut wanted = Embed::new(format!("🆘 {role_text} aranıyor: {title}"), COLOR_WARN)
                .url(&admin)
                .description(format!(
                    "**{name}** yeni saate katılamıyor. Maçı almak için: {admin}"
                ));
            if let Some(at) = info.starts_at {
                wanted = wanted.field("Maç saati", when(at), false);
            }
            // Pings everyone who could take the slot, as the site tells them.
            let staff = db::discord::declined_recipients(&cx.state.db, id, *user_id, *role).await?;
            cx.post_setting_pinging(channel, wanted, &staff);
            // They're already off the match, so this pings who's left.
            let mut p = Pings::default();
            let all = p.everyone(info, players);
            cx.post_match(
                id,
                format!(
                    "ℹ️ **{name}** yeni saatte {role_text} olamayacak, yerine biri bulunacak.\n{all}"
                ),
                p,
            );
        }
        Job::StaffChanged {
            role, user_id, by, ..
        } => {
            let role_text = role_name(*role);
            let icon = match role {
                StaffRole::Referee => "🧑‍⚖️",
                StaffRole::Streamer => "🎥",
            };
            let mut p = Pings::default();
            let newcomer = match user_id {
                Some(user_id) => user(cx.state, *user_id).await?,
                None => None,
            };
            let text = match &newcomer {
                Some(u) => format!(
                    "{icon} Bu maçın yeni {}: {}",
                    role_possessive(*role),
                    p.person(u.discord_id.as_deref(), &u.username)
                ),
                None => format!("ℹ️ Bu maçın artık {} yok.", role_possessive(*role)),
            };
            let all = p.everyone(info, players);
            cx.post_match(id, format!("{text}\n{all}"), p);
            // Someone a host put on the match also hears it directly.
            if let (Some(u), Some(user_id)) = (&newcomer, user_id) {
                if by != user_id {
                    let mut embed = Embed::new("📋 Maça atandın", COLOR)
                        .url(match_url(cx.state, info))
                        .description(format!("**{title}** maçına {role_text} olarak atandın."));
                    if let Some(at) = info.starts_at {
                        embed = embed.field("Maç saati", when(at), false);
                    }
                    cx.post_user(
                        u.discord_id.as_deref(),
                        OutMessage::embed(embed, Vec::new()),
                    );
                }
            }
        }
        Job::TimeSet { .. } => {
            let mut p = Pings::default();
            let all = p.everyone(info, players);
            let at = info.starts_at.map_or("belirsiz".to_string(), when);
            cx.post_match(id, format!("🕒 Maç saati güncellendi: **{at}**\n{all}"), p);
        }
        Job::ConfirmTime { .. } => {
            if let Some(at) = info.starts_at {
                ask_staff(cx, info, &title, at).await?;
            }
        }
        Job::RoomAdded { mp_id, .. } => {
            let mut p = Pings::default();
            let all = p.everyone(info, players);
            let what = if info.mp_ids.first() == Some(mp_id) {
                "Maç odası açıldı"
            } else {
                "Yeni maç odası açıldı"
            };
            cx.post_match(
                id,
                format!(
                    "🎮 {what}: {}\n{all} oyun içi davetleri kontrol edin!",
                    room_url(*mp_id)
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
            let mut p = Pings::default();
            let all = p.everyone(info, players);
            cx.post_match(id, format!("{text}\n{all}"), p);
        }
        Job::Reopened { .. } => {
            let mut p = Pings::default();
            let all = p.everyone(info, players);
            cx.post_match(id, format!("↩️ Maç yeniden açıldı.\n{all}"), p);
        }
        Job::Reminder { .. } => {
            let Some(at) = info.starts_at else {
                return Ok(());
            };
            let mut p = Pings::default();
            let all = p.everyone(info, players);
            let mut text = format!(
                "⏰ **{title}** yaklaşık 1 saat sonra başlıyor ({})\n{all}",
                when(at)
            );
            if info.referee_id.is_none() {
                text.push_str("\n⚠️ Bu maçın henüz hakemi yok.");
                cx.post_setting(
                    cx.settings.referee_channel_id,
                    Embed::new(format!("⚠️ Hakem yok: {title}"), COLOR_BAD)
                        .url(admin_url(cx.state, info))
                        .description(format!("Maç {} başlıyor ve hakemi yok!", when(at))),
                );
            }
            if info.kind == MatchKind::Bracket && info.streamer_id.is_none() {
                cx.post_setting(
                    cx.settings.streamer_channel_id,
                    Embed::new(format!("📺 Yayıncı yok: {title}"), COLOR_WARN)
                        .url(admin_url(cx.state, info))
                        .description(format!("Maç {} başlıyor ve yayıncısı yok.", when(at))),
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
                .map(|mp| format!("\nOda: {}", room_url(*mp)))
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

// The referee's and streamer's unanswered questions about the match's new time, as DMs
// with buttons that answer them like the site's notification does.
async fn ask_staff(
    cx: &mut Ctx<'_>,
    info: &MatchInfo,
    title: &str,
    new_starts_at: OffsetDateTime,
) -> BotResult {
    for request in db::discord::confirm_requests(&cx.state.db, info.id).await? {
        let role = role_name(request.staff_role);
        cx.post_user(
            Some(&request.discord_id),
            OutMessage {
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
                ..OutMessage::embed(
                    Embed::new(format!("🕒 Maç saati değişti: {title}"), COLOR_WARN)
                        .url(match_url(cx.state, info))
                        .description(format!(
                            "Bu saatte {role} olarak katılabilecek misin? Katılamazsan maçtan \
                             çıkarılırsın ve yerine biri aranır."
                        ))
                        .field("Yeni saat", when(new_starts_at), false),
                    Vec::new(),
                )
            },
        );
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
        url: Some(match_url(cx.state, info)),
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
        .map(|p| osu_profile(p.osu_id, &p.username))
        .collect();
    cx.post_hosts(
        Embed::new(format!("🤝 Yeni takım: {name}"), COLOR_GOOD).description(names.join(", ")),
    );

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
