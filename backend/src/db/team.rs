use serde::Serialize;
use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::role::Role;

// The public slice of an osu_users row that the team pages show.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Player {
    pub id: Uuid,
    pub osu_id: i64,
    pub username: String,
    pub avatar_url: String,
    pub country_code: Option<String>,
    pub global_rank: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PlayerState {
    #[sqlx(flatten)]
    pub player: Player,
    pub roles: Vec<Role>,
    pub in_team: bool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct SearchRow {
    #[sqlx(flatten)]
    pub state: PlayerState,
    pub invite_sent: bool,
    pub invite_received: bool,
}

// An invite together with the player on the other side of it.
#[derive(Debug, sqlx::FromRow)]
pub struct InviteRow {
    pub invite_id: Uuid,
    pub created_at: OffsetDateTime,
    pub team_name: String,
    #[sqlx(flatten)]
    pub state: PlayerState,
}

#[derive(Debug, sqlx::FromRow)]
pub struct TeamMemberRow {
    pub team_id: Uuid,
    pub team_name: String,
    pub team_created_at: OffsetDateTime,
    pub banner_updated_at: Option<OffsetDateTime>,
    #[sqlx(flatten)]
    pub player: Player,
    pub discord_id: Option<String>,
    pub discord_username: Option<String>,
    pub discord_in_guild: Option<bool>,
}

pub async fn find_player(
    ex: impl PgExecutor<'_>,
    user_id: Uuid,
) -> sqlx::Result<Option<PlayerState>> {
    sqlx::query_as::<_, PlayerState>(
        r#"
        SELECT u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank, u.roles,
               EXISTS(SELECT 1 FROM team_members m WHERE m.user_id = u.id) AS in_team
        FROM osu_users u
        WHERE u.id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(ex)
    .await
}

// Case-insensitive search over players (the roles that can play), exact matches first.
pub async fn search_players(
    pool: &PgPool,
    caller: Uuid,
    query: &str,
) -> sqlx::Result<Vec<SearchRow>> {
    let escaped = query
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    sqlx::query_as::<_, SearchRow>(
        r#"
        SELECT u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank, u.roles,
               EXISTS(SELECT 1 FROM team_members m WHERE m.user_id = u.id) AS in_team,
               EXISTS(SELECT 1 FROM team_invites i
                      WHERE i.inviter_id = $1 AND i.invitee_id = u.id) AS invite_sent,
               EXISTS(SELECT 1 FROM team_invites i
                      WHERE i.inviter_id = u.id AND i.invitee_id = $1) AS invite_received
        FROM osu_users u
        WHERE u.id <> $1 AND u.roles <@ '{streamer,graphics_designer,commentator}'::staff_role[] AND u.username ILIKE '%' || $2 || '%'
        ORDER BY lower(u.username) = lower($3) DESC, u.global_rank ASC NULLS LAST
        LIMIT 25
        "#,
    )
    .bind(caller)
    .bind(escaped)
    .bind(query)
    .fetch_all(pool)
    .await
}

pub async fn list_sent_invites(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<InviteRow>> {
    sqlx::query_as::<_, InviteRow>(
        r#"
        SELECT i.id AS invite_id, i.created_at, i.team_name,
               u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank, u.roles,
               EXISTS(SELECT 1 FROM team_members m WHERE m.user_id = u.id) AS in_team
        FROM team_invites i
        JOIN osu_users u ON u.id = i.invitee_id
        WHERE i.inviter_id = $1
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn list_received_invites(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Vec<InviteRow>> {
    sqlx::query_as::<_, InviteRow>(
        r#"
        SELECT i.id AS invite_id, i.created_at, i.team_name,
               u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank, u.roles,
               EXISTS(SELECT 1 FROM team_members m WHERE m.user_id = u.id) AS in_team
        FROM team_invites i
        JOIN osu_users u ON u.id = i.inviter_id
        WHERE i.invitee_id = $1
        ORDER BY i.created_at DESC
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

// Returns None when the invite already exists.
pub async fn create_invite(
    pool: &PgPool,
    inviter: Uuid,
    invitee: Uuid,
    team_name: &str,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        r#"
        INSERT INTO team_invites (inviter_id, invitee_id, team_name) VALUES ($1, $2, $3)
        ON CONFLICT (inviter_id, invitee_id) DO NOTHING
        RETURNING id
        "#,
    )
    .bind(inviter)
    .bind(invitee)
    .bind(team_name)
    .fetch_optional(pool)
    .await
}

pub async fn invite_exists(pool: &PgPool, inviter: Uuid, invitee: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM team_invites WHERE inviter_id = $1 AND invitee_id = $2)",
    )
    .bind(inviter)
    .bind(invitee)
    .fetch_one(pool)
    .await
}

// Deletes an invite the user sent (cancel) or received (decline).
pub async fn delete_invite(pool: &PgPool, invite_id: Uuid, user_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "DELETE FROM team_invites WHERE id = $1 AND (inviter_id = $2 OR invitee_id = $2)",
    )
    .bind(invite_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn received_invite_inviter(
    pool: &PgPool,
    invite_id: Uuid,
    invitee: Uuid,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT inviter_id FROM team_invites WHERE id = $1 AND invitee_id = $2")
        .bind(invite_id)
        .bind(invitee)
        .fetch_optional(pool)
        .await
}

// Locks a received invite for the accept transaction and returns its inviter and
// team name.
pub async fn lock_received_invite(
    conn: &mut PgConnection,
    invite_id: Uuid,
    invitee: Uuid,
) -> sqlx::Result<Option<(Uuid, String)>> {
    sqlx::query_as(
        r#"
        SELECT inviter_id, team_name FROM team_invites
        WHERE id = $1 AND invitee_id = $2
        FOR UPDATE
        "#,
    )
    .bind(invite_id)
    .bind(invitee)
    .fetch_optional(conn)
    .await
}

// Creates a team of two and drops every pending invite involving either player.
pub async fn create_team(
    conn: &mut PgConnection,
    a: Uuid,
    b: Uuid,
    name: &str,
) -> sqlx::Result<Uuid> {
    let team_id: Uuid = sqlx::query_scalar("INSERT INTO teams (name) VALUES ($1) RETURNING id")
        .bind(name)
        .fetch_one(&mut *conn)
        .await?;
    sqlx::query("INSERT INTO team_members (user_id, team_id) VALUES ($1, $3), ($2, $3)")
        .bind(a)
        .bind(b)
        .bind(team_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        r#"
        DELETE FROM team_invites
        WHERE inviter_id IN ($1, $2) OR invitee_id IN ($1, $2)
        "#,
    )
    .bind(a)
    .bind(b)
    .execute(&mut *conn)
    .await?;
    Ok(team_id)
}

pub async fn find_team_id(pool: &PgPool, user_id: Uuid) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar("SELECT team_id FROM team_members WHERE user_id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await
}

pub async fn rename_team(ex: impl PgExecutor<'_>, team_id: Uuid, name: &str) -> sqlx::Result<bool> {
    let result = sqlx::query("UPDATE teams SET name = $2 WHERE id = $1")
        .bind(team_id)
        .bind(name)
        .execute(ex)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn delete_team(ex: impl PgExecutor<'_>, team_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM teams WHERE id = $1")
        .bind(team_id)
        .execute(ex)
        .await?;
    Ok(result.rows_affected() > 0)
}

// Members of every team, or only of `user_id`'s team when given, grouped by team
// in name order.
pub async fn list_team_members(
    pool: &PgPool,
    user_id: Option<Uuid>,
) -> sqlx::Result<Vec<TeamMemberRow>> {
    sqlx::query_as::<_, TeamMemberRow>(
        r#"
        SELECT t.id AS team_id, t.name AS team_name, t.created_at AS team_created_at,
               b.updated_at AS banner_updated_at,
               u.id, u.osu_id, u.username, u.avatar_url, u.country_code, u.global_rank,
               d.discord_id, d.username AS discord_username, d.in_guild AS discord_in_guild
        FROM teams t
        LEFT JOIN team_banners b ON b.team_id = t.id
        JOIN team_members m ON m.team_id = t.id
        JOIN osu_users u ON u.id = m.user_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        WHERE $1::uuid IS NULL
           OR t.id = (SELECT team_id FROM team_members WHERE user_id = $1)
        ORDER BY lower(t.name), t.id, m.joined_at, u.username
        "#,
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct Banner {
    pub content_type: String,
    pub data: Vec<u8>,
}

pub async fn find_banner(pool: &PgPool, team_id: Uuid) -> sqlx::Result<Option<Banner>> {
    sqlx::query_as::<_, Banner>("SELECT content_type, data FROM team_banners WHERE team_id = $1")
        .bind(team_id)
        .fetch_optional(pool)
        .await
}

pub async fn set_banner(
    pool: &PgPool,
    team_id: Uuid,
    content_type: &str,
    data: &[u8],
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO team_banners (team_id, content_type, data) VALUES ($1, $2, $3)
        ON CONFLICT (team_id) DO UPDATE
        SET content_type = EXCLUDED.content_type, data = EXCLUDED.data, updated_at = now()
        "#,
    )
    .bind(team_id)
    .bind(content_type)
    .bind(data)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_banner(pool: &PgPool, team_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM team_banners WHERE team_id = $1")
        .bind(team_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}
