use serde::{Deserialize, Serialize};
use sqlx::{types::Json, PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{bracket::Side, matches::MatchKind};
use crate::discord::{jobs::Job, snowflake};

// What hosts pick on /admin/discord. Snowflakes go to the browser as strings, since
// they don't fit in a JavaScript number.
#[derive(Debug, Clone, Default, Serialize, Deserialize, sqlx::FromRow)]
#[serde(default)]
pub struct Settings {
    #[serde(with = "snowflake::option")]
    pub verified_role_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub player_role_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub team_announce_channel_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub match_announce_channel_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub results_channel_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub host_channel_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub referee_channel_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub streamer_channel_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub qualifier_category_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub qualifier_archive_category_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub bracket_category_id: Option<i64>,
    #[serde(with = "snowflake::option")]
    pub bracket_archive_category_id: Option<i64>,
}

pub async fn settings(ex: impl PgExecutor<'_>) -> sqlx::Result<Settings> {
    sqlx::query_as::<_, Settings>(
        r#"
        SELECT verified_role_id, player_role_id, team_announce_channel_id,
               match_announce_channel_id, results_channel_id, host_channel_id,
               referee_channel_id, streamer_channel_id, qualifier_category_id,
               qualifier_archive_category_id, bracket_category_id, bracket_archive_category_id
        FROM discord_settings
        "#,
    )
    .fetch_one(ex)
    .await
}

pub async fn save_settings(pool: &PgPool, s: &Settings) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE discord_settings
        SET verified_role_id = $1, player_role_id = $2, team_announce_channel_id = $3,
            match_announce_channel_id = $4, results_channel_id = $5, host_channel_id = $6,
            referee_channel_id = $7, streamer_channel_id = $8, qualifier_category_id = $9,
            qualifier_archive_category_id = $10, bracket_category_id = $11,
            bracket_archive_category_id = $12
        "#,
    )
    .bind(s.verified_role_id)
    .bind(s.player_role_id)
    .bind(s.team_announce_channel_id)
    .bind(s.match_announce_channel_id)
    .bind(s.results_channel_id)
    .bind(s.host_channel_id)
    .bind(s.referee_channel_id)
    .bind(s.streamer_channel_id)
    .bind(s.qualifier_category_id)
    .bind(s.qualifier_archive_category_id)
    .bind(s.bracket_category_id)
    .bind(s.bracket_archive_category_id)
    .execute(pool)
    .await?;
    Ok(())
}

// The categories the bot created when `base_id` filled up, oldest first.
pub async fn overflow_categories(pool: &PgPool, base_id: i64) -> sqlx::Result<Vec<i64>> {
    sqlx::query_scalar(
        "SELECT category_id FROM discord_overflow_categories WHERE base_id = $1 ORDER BY created_at",
    )
    .bind(base_id)
    .fetch_all(pool)
    .await
}

pub async fn add_overflow_category(
    pool: &PgPool,
    base_id: i64,
    category_id: i64,
) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO discord_overflow_categories (category_id, base_id) VALUES ($1, $2)")
        .bind(category_id)
        .bind(base_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ---------- the outbox ----------

#[derive(Debug, sqlx::FromRow)]
pub struct OutboxRow {
    pub id: i64,
    pub job: Json<serde_json::Value>,
    pub attempts: i32,
}

// Queues jobs in order. The notification wakes the worker once the transaction
// commits, so it never looks before the jobs are visible.
pub async fn push_jobs(ex: impl PgExecutor<'_>, jobs: &[Job]) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        WITH queued AS (
            INSERT INTO discord_outbox (job)
            SELECT j.job FROM jsonb_array_elements($1) WITH ORDINALITY AS j (job, n)
            ORDER BY j.n
            RETURNING id
        )
        SELECT pg_notify('discord_outbox', '') FROM queued LIMIT 1
        "#,
    )
    .bind(Json(jobs))
    .execute(ex)
    .await?;
    Ok(())
}

pub async fn next_job(pool: &PgPool) -> sqlx::Result<Option<OutboxRow>> {
    sqlx::query_as::<_, OutboxRow>(
        "SELECT id, job, attempts FROM discord_outbox WHERE run_after <= now() ORDER BY id LIMIT 1",
    )
    .fetch_optional(pool)
    .await
}

// How long until the next retry is due, if one is waiting.
pub async fn next_retry_in(pool: &PgPool) -> sqlx::Result<Option<f64>> {
    sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM min(run_after) - now())::float8 FROM discord_outbox",
    )
    .fetch_one(pool)
    .await
}

pub async fn finish_job(pool: &PgPool, id: i64) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM discord_outbox WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn retry_job(pool: &PgPool, id: i64, after_secs: i64) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE discord_outbox
        SET attempts = attempts + 1, run_after = now() + make_interval(secs => $2)
        WHERE id = $1
        "#,
    )
    .bind(id)
    .bind(after_secs as f64)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------- accounts ----------

#[derive(Debug, sqlx::FromRow)]
pub struct LinkedAccount {
    pub user_id: Uuid,
    pub discord_id: String,
    pub in_guild: Option<bool>,
    pub in_team: bool,
}

pub async fn linked_accounts(pool: &PgPool) -> sqlx::Result<Vec<LinkedAccount>> {
    sqlx::query_as::<_, LinkedAccount>(
        r#"
        SELECT d.osu_user_id AS user_id, d.discord_id, d.in_guild,
               EXISTS (SELECT 1 FROM team_members tm WHERE tm.user_id = d.osu_user_id) AS in_team
        FROM discord_accounts d
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn find_linked(pool: &PgPool, discord_id: &str) -> sqlx::Result<Option<LinkedAccount>> {
    sqlx::query_as::<_, LinkedAccount>(
        r#"
        SELECT d.osu_user_id AS user_id, d.discord_id, d.in_guild,
               EXISTS (SELECT 1 FROM team_members tm WHERE tm.user_id = d.osu_user_id) AS in_team
        FROM discord_accounts d
        WHERE d.discord_id = $1
        "#,
    )
    .bind(discord_id)
    .fetch_optional(pool)
    .await
}

// The user's Discord account and whether it's in the server, if linked.
pub async fn account_of(
    ex: impl PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<Option<(String, Option<bool>)>> {
    sqlx::query_as("SELECT discord_id, in_guild FROM discord_accounts WHERE osu_user_id = $1")
        .bind(user_id)
        .fetch_optional(ex)
        .await
}

// Records whether an account is in the server and returns its user with what was
// recorded before, or None when nobody linked that account.
pub async fn set_in_guild(
    pool: &PgPool,
    discord_id: &str,
    in_guild: bool,
) -> sqlx::Result<Option<(Uuid, Option<bool>)>> {
    sqlx::query_as(
        r#"
        UPDATE discord_accounts d SET in_guild = $2
        FROM (SELECT id, in_guild FROM discord_accounts WHERE discord_id = $1 FOR UPDATE) old
        WHERE d.id = old.id
        RETURNING d.osu_user_id, old.in_guild
        "#,
    )
    .bind(discord_id)
    .bind(in_guild)
    .fetch_optional(pool)
    .await
}

// Tells a team member's teammate and the hosts that they left the server.
pub async fn notify_left(pool: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, kind, team_id, actor_id)
        SELECT r.id, 'discord_left', tm.team_id, $1
        FROM team_members tm
        CROSS JOIN LATERAL (
            SELECT o.user_id AS id FROM team_members o
            WHERE o.team_id = tm.team_id AND o.user_id <> $1
            UNION
            SELECT u.id FROM osu_users u WHERE u.role = 'host'
        ) AS r
        WHERE tm.user_id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

// The user's unconcluded matches as a player, referee or streamer, so their channels
// can be updated when they come back to the server.
pub async fn open_matches_of(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        WHERE m.concluded_at IS NULL AND (
            m.referee_id = $1 OR b.streamer_id = $1
            OR EXISTS (SELECT 1 FROM team_members tm
                       WHERE tm.user_id = $1 AND tm.team_id IN (b.team1_id, b.team2_id))
            OR EXISTS (SELECT 1 FROM team_members tm
                       JOIN qualifier_entries e ON e.team_id = tm.team_id
                       WHERE tm.user_id = $1 AND e.match_id = m.id)
        )
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

// Every match a team plays or is signed up for that isn't concluded.
pub async fn open_matches_of_team(
    ex: impl PgExecutor<'_>,
    team_id: Uuid,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        WHERE m.concluded_at IS NULL
          AND ($1 IN (b.team1_id, b.team2_id)
               OR EXISTS (SELECT 1 FROM qualifier_entries e
                          WHERE e.team_id = $1 AND e.match_id = m.id))
        "#,
    )
    .bind(team_id)
    .fetch_all(ex)
    .await
}

// ---------- matches ----------

// A match with everything the bot writes about it.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MatchInfo {
    pub id: Uuid,
    pub kind: MatchKind,
    pub starts_at: Option<OffsetDateTime>,
    pub concluded_at: Option<OffsetDateTime>,
    pub side: Option<Side>,
    pub number: Option<i32>,
    pub lobby_number: Option<i64>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub team1_name: Option<String>,
    pub team2_name: Option<String>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub referee_id: Option<Uuid>,
    pub referee_username: Option<String>,
    pub referee_discord_id: Option<String>,
    pub referee_in_guild: Option<bool>,
    pub streamer_id: Option<Uuid>,
    pub streamer_username: Option<String>,
    pub streamer_discord_id: Option<String>,
    pub streamer_in_guild: Option<bool>,
    pub stage_id: Option<Uuid>,
    pub stage_name: Option<String>,
    pub mp_ids: Vec<i64>,
    pub entries: i64,
}

pub async fn match_info(ex: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<Option<MatchInfo>> {
    sqlx::query_as::<_, MatchInfo>(
        r#"
        SELECT m.id, m.kind, m.starts_at, m.concluded_at,
               b.side, b.number,
               CASE WHEN m.kind = 'qualifier' THEN
                   (SELECT count(*) FROM matches q
                    WHERE q.kind = 'qualifier'
                      AND (q.starts_at, q.created_at) <= (m.starts_at, m.created_at))
               END AS lobby_number,
               b.team1_id, b.team2_id, t1.name AS team1_name, t2.name AS team2_name,
               b.team1_score, b.team2_score,
               m.referee_id, r.username AS referee_username,
               rd.discord_id AS referee_discord_id, rd.in_guild AS referee_in_guild,
               b.streamer_id, s.username AS streamer_username,
               sd.discord_id AS streamer_discord_id, sd.in_guild AS streamer_in_guild,
               st.id AS stage_id, st.name AS stage_name,
               ARRAY(SELECT mr.mp_id FROM match_rooms mr WHERE mr.match_id = m.id
                     ORDER BY mr.added_at, mr.mp_id) AS mp_ids,
               (SELECT count(*) FROM qualifier_entries e WHERE e.match_id = m.id) AS entries
        FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        LEFT JOIN teams t1 ON t1.id = b.team1_id
        LEFT JOIN teams t2 ON t2.id = b.team2_id
        LEFT JOIN osu_users r ON r.id = m.referee_id
        LEFT JOIN discord_accounts rd ON rd.osu_user_id = r.id
        LEFT JOIN osu_users s ON s.id = b.streamer_id
        LEFT JOIN discord_accounts sd ON sd.osu_user_id = s.id
        LEFT JOIN stages st ON st.id = CASE
            WHEN m.kind = 'qualifier' THEN (SELECT x.id FROM stages x WHERE x.is_qualifier)
            ELSE b.stage_id
        END
        WHERE m.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(ex)
    .await
}

// A player of a match: a bracket match's two teams or a lobby's signed-up ones.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MatchPlayer {
    pub team_id: Uuid,
    pub team_name: String,
    pub user_id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub global_rank: Option<i64>,
    pub discord_id: Option<String>,
    pub in_guild: Option<bool>,
}

// In team order (team 1 first, or signup order for a lobby), then join order.
pub async fn match_players(
    ex: impl PgExecutor<'_>,
    match_id: Uuid,
) -> sqlx::Result<Vec<MatchPlayer>> {
    sqlx::query_as::<_, MatchPlayer>(
        r#"
        SELECT t.id AS team_id, t.name AS team_name, u.id AS user_id, u.osu_id, u.username,
               u.global_rank, d.discord_id, d.in_guild
        FROM (
            SELECT b.team1_id AS team_id, 1 AS n, now() AS joined FROM bracket_matches b WHERE b.match_id = $1
            UNION ALL
            SELECT b.team2_id, 2, now() FROM bracket_matches b WHERE b.match_id = $1
            UNION ALL
            SELECT e.team_id, 3, e.joined_at FROM qualifier_entries e WHERE e.match_id = $1
        ) AS mt
        JOIN teams t ON t.id = mt.team_id
        JOIN team_members tm ON tm.team_id = t.id
        JOIN osu_users u ON u.id = tm.user_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        ORDER BY mt.n, mt.joined, t.id, tm.joined_at, u.username
        "#,
    )
    .bind(match_id)
    .fetch_all(ex)
    .await
}

#[derive(Debug, Default, sqlx::FromRow)]
pub struct MatchDiscord {
    pub channel_id: Option<i64>,
    pub synced: Option<String>,
    pub results_channel_id: Option<i64>,
    pub results_message_id: Option<i64>,
}

pub async fn match_discord(ex: impl PgExecutor<'_>, match_id: Uuid) -> sqlx::Result<MatchDiscord> {
    let row = sqlx::query_as::<_, MatchDiscord>(
        r#"
        SELECT channel_id, synced, results_channel_id, results_message_id
        FROM match_discord WHERE match_id = $1
        "#,
    )
    .bind(match_id)
    .fetch_optional(ex)
    .await?;
    Ok(row.unwrap_or_default())
}

pub async fn channel_of(ex: impl PgExecutor<'_>, match_id: Uuid) -> sqlx::Result<Option<i64>> {
    let id: Option<Option<i64>> =
        sqlx::query_scalar("SELECT channel_id FROM match_discord WHERE match_id = $1")
            .bind(match_id)
            .fetch_optional(ex)
            .await?;
    Ok(id.flatten())
}

pub async fn set_channel(
    pool: &PgPool,
    match_id: Uuid,
    channel_id: i64,
    synced: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO match_discord (match_id, channel_id, synced) VALUES ($1, $2, $3)
        ON CONFLICT (match_id) DO UPDATE SET channel_id = EXCLUDED.channel_id, synced = EXCLUDED.synced
        "#,
    )
    .bind(match_id)
    .bind(channel_id)
    .bind(synced)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn set_results(
    pool: &PgPool,
    match_id: Uuid,
    message: Option<(i64, i64)>,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO match_discord (match_id, results_channel_id, results_message_id) VALUES ($1, $2, $3)
        ON CONFLICT (match_id) DO UPDATE
        SET results_channel_id = EXCLUDED.results_channel_id,
            results_message_id = EXCLUDED.results_message_id
        "#,
    )
    .bind(match_id)
    .bind(message.map(|m| m.0))
    .bind(message.map(|m| m.1))
    .execute(pool)
    .await?;
    Ok(())
}

// Every match that has a channel or should get one, for a full resync.
pub async fn syncable_matches(pool: &PgPool) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        LEFT JOIN match_discord md ON md.match_id = m.id
        WHERE md.channel_id IS NOT NULL
           OR (m.concluded_at IS NULL AND m.kind = 'bracket'
               AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL AND m.starts_at IS NOT NULL)
           OR (m.concluded_at IS NULL AND m.kind = 'qualifier'
               AND EXISTS (SELECT 1 FROM qualifier_entries e WHERE e.match_id = m.id))
        ORDER BY m.starts_at NULLS LAST
        "#,
    )
    .fetch_all(pool)
    .await
}

// Lobby numbers follow start times, so moving one lobby can renumber the others.
pub async fn lobby_ids(ex: impl PgExecutor<'_>) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar("SELECT id FROM matches WHERE kind = 'qualifier'")
        .fetch_all(ex)
        .await
}

// Matches starting within `window` that haven't been reminded about at this time yet:
// bracket matches with both teams, and lobbies with a team. Marks them reminded.
pub async fn take_due_reminders(
    conn: &mut PgConnection,
    window_secs: i64,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        INSERT INTO match_discord (match_id, reminded_for)
        SELECT m.id, m.starts_at FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        LEFT JOIN match_discord md ON md.match_id = m.id
        WHERE m.concluded_at IS NULL
          AND m.starts_at > now() AND m.starts_at <= now() + make_interval(secs => $1)
          AND md.reminded_for IS DISTINCT FROM m.starts_at
          AND CASE m.kind
                  WHEN 'bracket' THEN b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
                  ELSE EXISTS (SELECT 1 FROM qualifier_entries e WHERE e.match_id = m.id)
              END
        ON CONFLICT (match_id) DO UPDATE SET reminded_for = EXCLUDED.reminded_for
        RETURNING match_id
        "#,
    )
    .bind(window_secs as f64)
    .fetch_all(conn)
    .await
}

// Bracket matches with both teams starting within `window` that haven't been announced
// for this time yet. Marks them announced.
pub async fn take_due_announcements(
    conn: &mut PgConnection,
    window_secs: i64,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        INSERT INTO match_discord (match_id, announced_for)
        SELECT m.id, m.starts_at FROM matches m
        JOIN bracket_matches b ON b.match_id = m.id
        LEFT JOIN match_discord md ON md.match_id = m.id
        WHERE m.concluded_at IS NULL
          AND m.starts_at > now() AND m.starts_at <= now() + make_interval(secs => $1)
          AND md.announced_for IS DISTINCT FROM m.starts_at
          AND b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
        ON CONFLICT (match_id) DO UPDATE SET announced_for = EXCLUDED.announced_for
        RETURNING match_id
        "#,
    )
    .bind(window_secs as f64)
    .fetch_all(conn)
    .await
}

// ---------- subscriptions ----------

pub async fn subscriptions(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar("SELECT match_id FROM match_subscriptions WHERE user_id = $1")
        .bind(user_id)
        .fetch_all(pool)
        .await
}

pub async fn subscribe(pool: &PgPool, match_id: Uuid, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO match_subscriptions (match_id, user_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(match_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unsubscribe(pool: &PgPool, match_id: Uuid, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("DELETE FROM match_subscriptions WHERE match_id = $1 AND user_id = $2")
        .bind(match_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// The Discord accounts of a match's subscribers that are in the server.
pub async fn subscriber_discord_ids(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar(
        r#"
        SELECT d.discord_id FROM match_subscriptions s
        JOIN discord_accounts d ON d.osu_user_id = s.user_id
        WHERE s.match_id = $1 AND d.in_guild
        ORDER BY s.created_at
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

// ---------- team announcements ----------

// Records the players as announced now, unless both already were within the week.
// True when the team should be announced.
pub async fn take_team_announcement(
    conn: &mut PgConnection,
    user_ids: &[Uuid],
) -> sqlx::Result<bool> {
    let recent: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*) FROM discord_announced_players
        WHERE user_id = ANY($1) AND announced_at > now() - interval '7 days'
        "#,
    )
    .bind(user_ids)
    .fetch_one(&mut *conn)
    .await?;
    if recent >= user_ids.len() as i64 {
        return Ok(false);
    }
    sqlx::query(
        r#"
        INSERT INTO discord_announced_players (user_id, announced_at)
        SELECT unnest($1::uuid[]), now()
        ON CONFLICT (user_id) DO UPDATE SET announced_at = EXCLUDED.announced_at
        "#,
    )
    .bind(user_ids)
    .execute(conn)
    .await?;
    Ok(true)
}

// ---------- slash commands ----------

#[derive(Debug, sqlx::FromRow)]
pub struct TeamName {
    pub id: Uuid,
    pub name: String,
}

// Teams whose name contains the text, best matches first.
pub async fn search_teams(pool: &PgPool, query: &str) -> sqlx::Result<Vec<TeamName>> {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    sqlx::query_as::<_, TeamName>(
        r#"
        SELECT id, name FROM teams
        WHERE name ILIKE '%' || $1 || '%'
        ORDER BY lower(name) = lower($2) DESC, lower(name) LIKE lower($2) || '%' DESC, lower(name)
        LIMIT 25
        "#,
    )
    .bind(escaped)
    .bind(query)
    .fetch_all(pool)
    .await
}

pub async fn find_team_by_name(pool: &PgPool, name: &str) -> sqlx::Result<Option<TeamName>> {
    sqlx::query_as::<_, TeamName>("SELECT id, name FROM teams WHERE lower(name) = lower($1)")
        .bind(name)
        .fetch_optional(pool)
        .await
}

pub async fn team_of_discord(pool: &PgPool, discord_id: &str) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT tm.team_id FROM discord_accounts d
        JOIN team_members tm ON tm.user_id = d.osu_user_id
        WHERE d.discord_id = $1
        "#,
    )
    .bind(discord_id)
    .fetch_optional(pool)
    .await
}

// Unconcluded matches still to come, soonest first: bracket matches with both teams
// and lobbies, or only the team's when one is given.
pub async fn upcoming_matches(
    pool: &PgPool,
    team_id: Option<Uuid>,
    limit: i64,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        WHERE m.concluded_at IS NULL AND m.starts_at > now() - interval '3 hours'
          AND CASE m.kind
                  WHEN 'bracket' THEN b.team1_id IS NOT NULL AND b.team2_id IS NOT NULL
                       AND ($1::uuid IS NULL OR $1 IN (b.team1_id, b.team2_id))
                  ELSE $1::uuid IS NULL OR EXISTS (
                       SELECT 1 FROM qualifier_entries e WHERE e.match_id = m.id AND e.team_id = $1)
              END
        ORDER BY m.starts_at, m.created_at
        LIMIT $2
        "#,
    )
    .bind(team_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

// A team's concluded bracket matches, latest first.
pub async fn team_history(pool: &PgPool, team_id: Uuid, limit: i64) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        SELECT m.id FROM matches m
        JOIN bracket_matches b ON b.match_id = m.id
        WHERE m.concluded_at IS NOT NULL AND $1 IN (b.team1_id, b.team2_id)
        ORDER BY m.concluded_at DESC
        LIMIT $2
        "#,
    )
    .bind(team_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub async fn bracket_match_by_code(
    pool: &PgPool,
    side: Side,
    number: i32,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT match_id FROM bracket_matches WHERE side = $1 AND number = $2")
        .bind(side)
        .bind(number)
        .fetch_optional(pool)
        .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct MatchCode {
    pub side: Side,
    pub number: i32,
    pub team1_name: Option<String>,
    pub team2_name: Option<String>,
}

pub async fn match_codes(pool: &PgPool) -> sqlx::Result<Vec<MatchCode>> {
    sqlx::query_as::<_, MatchCode>(
        r#"
        SELECT b.side, b.number, t1.name AS team1_name, t2.name AS team2_name
        FROM bracket_matches b
        LEFT JOIN teams t1 ON t1.id = b.team1_id
        LEFT JOIN teams t2 ON t2.id = b.team2_id
        ORDER BY b.side, b.number
        "#,
    )
    .fetch_all(pool)
    .await
}

// A pool map as the results list it: its slot label and beatmap.
#[derive(Debug, sqlx::FromRow)]
pub struct ResultMap {
    pub map_id: Uuid,
    pub beatmap_id: i64,
    pub label: String,
    pub title: String,
    pub version: String,
}

pub async fn stage_maps(pool: &PgPool, stage_id: Uuid) -> sqlx::Result<Vec<ResultMap>> {
    sqlx::query_as::<_, ResultMap>(
        r#"
        SELECT map_id, beatmap_id, label, title, version FROM (
            SELECT pm.id AS map_id, pm.beatmap_id, pm.title, pm.version, c.position, s.position AS slot_position,
                   CASE WHEN count(*) OVER (PARTITION BY c.id) > 1
                        THEN c.name || (row_number() OVER (PARTITION BY c.id ORDER BY s.position))::text
                        ELSE c.name
                   END AS label
            FROM stage_categories c
            JOIN category_slots s ON s.category_id = c.id
            LEFT JOIN pool_maps pm ON pm.slot_id = s.id
            WHERE c.stage_id = $1
        ) AS slots
        WHERE map_id IS NOT NULL
        ORDER BY position, slot_position
        "#,
    )
    .bind(stage_id)
    .fetch_all(pool)
    .await
}

pub async fn clear_channel(pool: &PgPool, match_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE match_discord SET channel_id = NULL, synced = NULL WHERE match_id = $1")
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(())
}

// A team's players, in join order.
pub async fn team_players(
    ex: impl PgExecutor<'_>,
    team_id: Uuid,
) -> sqlx::Result<Vec<MatchPlayer>> {
    sqlx::query_as::<_, MatchPlayer>(
        r#"
        SELECT t.id AS team_id, t.name AS team_name, u.id AS user_id, u.osu_id, u.username,
               u.global_rank, d.discord_id, d.in_guild
        FROM teams t
        JOIN team_members tm ON tm.team_id = t.id
        JOIN osu_users u ON u.id = tm.user_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        WHERE t.id = $1
        ORDER BY tm.joined_at, u.username
        "#,
    )
    .bind(team_id)
    .fetch_all(ex)
    .await
}

// A user as the bot names them, with their team if they're in one.
#[derive(Debug, sqlx::FromRow)]
pub struct UserBrief {
    pub id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub discord_id: Option<String>,
    pub discord_username: Option<String>,
    pub team_id: Option<Uuid>,
    pub team_name: Option<String>,
}

pub async fn users_brief(pool: &PgPool, ids: &[Uuid]) -> sqlx::Result<Vec<UserBrief>> {
    sqlx::query_as::<_, UserBrief>(
        r#"
        SELECT u.id, u.osu_id, u.username, d.discord_id, d.username AS discord_username,
               t.id AS team_id, t.name AS team_name
        FROM osu_users u
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        LEFT JOIN team_members tm ON tm.user_id = u.id
        LEFT JOIN teams t ON t.id = tm.team_id
        WHERE u.id = ANY($1)
        "#,
    )
    .bind(ids)
    .fetch_all(pool)
    .await
}

// The unanswered requests to confirm a match's current time, with who they went to.
#[derive(Debug, sqlx::FromRow)]
pub struct ConfirmRequest {
    pub id: Uuid,
    pub staff_role: super::notifications::StaffRole,
    pub discord_id: String,
}

pub async fn confirm_requests(pool: &PgPool, match_id: Uuid) -> sqlx::Result<Vec<ConfirmRequest>> {
    sqlx::query_as::<_, ConfirmRequest>(
        r#"
        SELECT n.id, n.staff_role, d.discord_id
        FROM notifications n
        JOIN matches m ON m.id = n.match_id
        JOIN discord_accounts d ON d.osu_user_id = n.user_id
        WHERE n.match_id = $1 AND n.kind = 'confirm_time' AND n.response IS NULL
          AND n.new_starts_at = m.starts_at
        "#,
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
}

// When a team's banner was last changed, for its versioned URL.
pub async fn banner_version(pool: &PgPool, team_id: Uuid) -> sqlx::Result<Option<OffsetDateTime>> {
    sqlx::query_scalar("SELECT updated_at FROM team_banners WHERE team_id = $1")
        .bind(team_id)
        .fetch_optional(pool)
        .await
}
