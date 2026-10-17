use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{bracket::Side, team::Player};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, sqlx::Type)]
#[sqlx(type_name = "match_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MatchKind {
    Qualifier,
    Bracket,
}

// A match's referee, selected as referee_* columns. All null when nobody has taken
// the match.
#[derive(Debug, sqlx::FromRow)]
pub struct RefereeColumns {
    pub referee_id: Option<Uuid>,
    pub referee_osu_id: Option<i64>,
    pub referee_username: Option<String>,
    pub referee_avatar_url: Option<String>,
    pub referee_discord_id: Option<String>,
    pub referee_discord_username: Option<String>,
}

// A qualifier lobby with its referee folded in.
#[derive(Debug, sqlx::FromRow)]
pub struct MatchRow {
    pub id: Uuid,
    pub starts_at: OffsetDateTime,
    // Its rooms, oldest first.
    pub mp_ids: Vec<i64>,
    pub referee_notes: String,
    pub concluded_at: Option<OffsetDateTime>,
    #[sqlx(flatten)]
    pub referee: RefereeColumns,
}

// What the match actions check before changing a match.
#[derive(Debug, sqlx::FromRow)]
pub struct MatchState {
    pub kind: MatchKind,
    pub referee_id: Option<Uuid>,
    // Its rooms, oldest first.
    pub mp_ids: Vec<i64>,
    pub concluded_at: Option<OffsetDateTime>,
}

impl MatchState {
    pub fn concluded(&self) -> bool {
        self.concluded_at.is_some()
    }
}

// The qualifier lobby state a team signup checks, read under a row lock.
#[derive(Debug, sqlx::FromRow)]
pub struct LobbyLock {
    pub starts_at: OffsetDateTime,
    pub concluded_at: Option<OffsetDateTime>,
}

impl LobbyLock {
    // Players can only join or leave a lobby that hasn't started or been concluded.
    pub fn open(&self, now: OffsetDateTime) -> bool {
        self.concluded_at.is_none() && self.starts_at > now
    }
}

#[derive(Debug, sqlx::FromRow)]
pub struct EntryRow {
    pub team_id: Uuid,
    pub match_id: Uuid,
}

// A match as the referee page shows it. The bracket_* fields are null for
// qualifiers and lobby_number is null for bracket matches. The stage is the bracket
// match's own, or the qualifier stage for a lobby.
#[derive(Debug, sqlx::FromRow)]
pub struct RoomRow {
    pub id: Uuid,
    pub kind: MatchKind,
    pub starts_at: Option<OffsetDateTime>,
    // Its rooms, oldest first.
    pub mp_ids: Vec<i64>,
    pub referee_notes: String,
    pub concluded_at: Option<OffsetDateTime>,
    #[sqlx(flatten)]
    pub referee: RefereeColumns,
    pub side: Option<Side>,
    pub number: Option<i32>,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub forfeit_slot: Option<i16>,
    // Bracket matches only: who streams it, all null when nobody does.
    pub streamer_id: Option<Uuid>,
    pub streamer_osu_id: Option<i64>,
    pub streamer_username: Option<String>,
    pub streamer_avatar_url: Option<String>,
    pub lobby_number: Option<i64>,
    pub stage_id: Option<Uuid>,
    pub stage_name: Option<String>,
    pub stage_published: Option<bool>,
}

// Every qualifier lobby, earliest first.
pub async fn list_lobbies(pool: &PgPool) -> sqlx::Result<Vec<MatchRow>> {
    sqlx::query_as::<_, MatchRow>(
        r#"
        SELECT m.id, m.starts_at, m.referee_notes, m.concluded_at,
               ARRAY(SELECT r.mp_id FROM match_rooms r WHERE r.match_id = m.id
                     ORDER BY r.added_at, r.mp_id) AS mp_ids,
               u.id AS referee_id, u.osu_id AS referee_osu_id,
               u.username AS referee_username, u.avatar_url AS referee_avatar_url,
               d.discord_id AS referee_discord_id, d.username AS referee_discord_username
        FROM matches m
        LEFT JOIN osu_users u ON u.id = m.referee_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        WHERE m.kind = 'qualifier'
        ORDER BY m.starts_at, m.created_at
        "#,
    )
    .fetch_all(pool)
    .await
}

// Every qualifier signup, in the order teams joined their lobby.
pub async fn list_entries(pool: &PgPool) -> sqlx::Result<Vec<EntryRow>> {
    sqlx::query_as::<_, EntryRow>(
        "SELECT team_id, match_id FROM qualifier_entries ORDER BY joined_at, team_id",
    )
    .fetch_all(pool)
    .await
}

pub async fn create_lobby(pool: &PgPool, starts_at: OffsetDateTime) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        "INSERT INTO matches (kind, starts_at) VALUES ('qualifier', $1) RETURNING id",
    )
    .bind(starts_at)
    .fetch_one(pool)
    .await
}

pub async fn reschedule_lobby(
    pool: &PgPool,
    id: Uuid,
    starts_at: OffsetDateTime,
) -> sqlx::Result<bool> {
    let result =
        sqlx::query("UPDATE matches SET starts_at = $2 WHERE id = $1 AND kind = 'qualifier'")
            .bind(id)
            .bind(starts_at)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

// Deleting a lobby drops its signups, so its teams can pick another one.
pub async fn delete_lobby(pool: &PgPool, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM matches WHERE id = $1 AND kind = 'qualifier'")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

// Locks a qualifier lobby so signups to it run one at a time and can't overfill it.
pub async fn lock_lobby(conn: &mut PgConnection, id: Uuid) -> sqlx::Result<Option<LobbyLock>> {
    sqlx::query_as::<_, LobbyLock>(
        r#"
        SELECT starts_at, concluded_at FROM matches
        WHERE id = $1 AND kind = 'qualifier'
        FOR UPDATE
        "#,
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

// The lobby a team is signed up for, if any, with that lobby's state. Locks the
// signup so both players can't move their team at the same time.
pub async fn lock_team_lobby(
    conn: &mut PgConnection,
    team_id: Uuid,
) -> sqlx::Result<Option<(Uuid, LobbyLock)>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        match_id: Uuid,
        #[sqlx(flatten)]
        lobby: LobbyLock,
    }
    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT e.match_id, m.starts_at, m.concluded_at
        FROM qualifier_entries e
        JOIN matches m ON m.id = e.match_id
        WHERE e.team_id = $1
        FOR UPDATE OF e
        "#,
    )
    .bind(team_id)
    .fetch_optional(conn)
    .await?;
    Ok(row.map(|r| (r.match_id, r.lobby)))
}

pub async fn count_entries(conn: &mut PgConnection, match_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar("SELECT count(*) FROM qualifier_entries WHERE match_id = $1")
        .bind(match_id)
        .fetch_one(conn)
        .await
}

// Signs a team up for a lobby, moving it out of any lobby it was in.
pub async fn set_entry(conn: &mut PgConnection, team_id: Uuid, match_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO qualifier_entries (team_id, match_id) VALUES ($1, $2)
        ON CONFLICT (team_id) DO UPDATE SET match_id = EXCLUDED.match_id, joined_at = now()
        "#,
    )
    .bind(team_id)
    .bind(match_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn delete_entry(ex: impl PgExecutor<'_>, team_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM qualifier_entries WHERE team_id = $1")
        .bind(team_id)
        .execute(ex)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn find_match(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<MatchState>> {
    sqlx::query_as::<_, MatchState>(
        r#"
        SELECT m.kind, m.referee_id, m.concluded_at,
               ARRAY(SELECT r.mp_id FROM match_rooms r WHERE r.match_id = m.id
                     ORDER BY r.added_at, r.mp_id) AS mp_ids
        FROM matches m
        WHERE m.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

// Lobbies are numbered in the order list_lobbies returns them.
pub async fn find_room(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<RoomRow>> {
    sqlx::query_as::<_, RoomRow>(
        r#"
        SELECT m.id, m.kind, m.starts_at, m.referee_notes, m.concluded_at,
               ARRAY(SELECT r.mp_id FROM match_rooms r WHERE r.match_id = m.id
                     ORDER BY r.added_at, r.mp_id) AS mp_ids,
               u.id AS referee_id, u.osu_id AS referee_osu_id,
               u.username AS referee_username, u.avatar_url AS referee_avatar_url,
               d.discord_id AS referee_discord_id, d.username AS referee_discord_username,
               b.side, b.number, b.team1_id, b.team2_id, b.team1_score, b.team2_score,
               b.forfeit_slot,
               s.id AS streamer_id, s.osu_id AS streamer_osu_id,
               s.username AS streamer_username, s.avatar_url AS streamer_avatar_url,
               CASE WHEN m.kind = 'qualifier' THEN
                   (SELECT count(*) FROM matches q
                    WHERE q.kind = 'qualifier'
                      AND (q.starts_at, q.created_at) <= (m.starts_at, m.created_at))
               END AS lobby_number,
               st.id AS stage_id, st.name AS stage_name, st.published AS stage_published
        FROM matches m
        LEFT JOIN bracket_matches b ON b.match_id = m.id
        LEFT JOIN osu_users u ON u.id = m.referee_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        LEFT JOIN osu_users s ON s.id = b.streamer_id
        LEFT JOIN stages st ON st.id = CASE
            WHEN m.kind = 'qualifier' THEN (SELECT s.id FROM stages s WHERE s.is_qualifier)
            ELSE b.stage_id
        END
        WHERE m.id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

// Takes an unrefereed match. False when someone else got there first.
pub async fn claim_match(pool: &PgPool, id: Uuid, referee_id: Uuid) -> sqlx::Result<bool> {
    let result =
        sqlx::query("UPDATE matches SET referee_id = $2 WHERE id = $1 AND referee_id IS NULL")
            .bind(id)
            .bind(referee_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

// Drops the referee only if it's still `referee_id`.
pub async fn release_match(pool: &PgPool, id: Uuid, referee_id: Uuid) -> sqlx::Result<bool> {
    let result =
        sqlx::query("UPDATE matches SET referee_id = NULL WHERE id = $1 AND referee_id = $2")
            .bind(id)
            .bind(referee_id)
            .execute(pool)
            .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_referee(pool: &PgPool, id: Uuid, referee_id: Option<Uuid>) -> sqlx::Result<()> {
    sqlx::query("UPDATE matches SET referee_id = $2 WHERE id = $1")
        .bind(id)
        .bind(referee_id)
        .execute(pool)
        .await?;
    Ok(())
}

// Adds a room to a match. Fails on match_rooms_pkey when the room already belongs to
// a match.
pub async fn add_room(pool: &PgPool, match_id: Uuid, mp_id: i64) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO match_rooms (mp_id, match_id) VALUES ($1, $2)")
        .bind(mp_id)
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(())
}

// Takes a room off a match. Its saved chat stays, for moderation.
pub async fn remove_room(pool: &PgPool, match_id: Uuid, mp_id: i64) -> sqlx::Result<bool> {
    let result = sqlx::query("DELETE FROM match_rooms WHERE mp_id = $1 AND match_id = $2")
        .bind(mp_id)
        .bind(match_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_notes(pool: &PgPool, id: Uuid, notes: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE matches SET referee_notes = $2 WHERE id = $1")
        .bind(id)
        .bind(notes)
        .execute(pool)
        .await?;
    Ok(())
}

// Concludes a match that has a room. False when it's already concluded or has no
// room.
pub async fn conclude(ex: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        r#"
        UPDATE matches SET concluded_at = now()
        WHERE id = $1 AND concluded_at IS NULL
          AND EXISTS (SELECT 1 FROM match_rooms r WHERE r.match_id = $1)
        "#,
    )
    .bind(id)
    .execute(ex)
    .await?;
    Ok(result.rows_affected() > 0)
}

// Concludes a bracket match decided by forfeit, which may never have had a room.
pub async fn conclude_forfeit(ex: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE matches SET concluded_at = now() WHERE id = $1 AND concluded_at IS NULL",
    )
    .bind(id)
    .execute(ex)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn reopen(ex: impl PgExecutor<'_>, id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE matches SET concluded_at = NULL WHERE id = $1")
        .bind(id)
        .execute(ex)
        .await?;
    Ok(())
}

// Everyone who can referee: the users holding the referee role.
pub async fn list_referees(pool: &PgPool) -> sqlx::Result<Vec<Player>> {
    sqlx::query_as::<_, Player>(
        r#"
        SELECT id, osu_id, username, avatar_url, country_code, global_rank
        FROM osu_users
        WHERE 'referee' = ANY(roles)
        ORDER BY lower(username)
        "#,
    )
    .fetch_all(pool)
    .await
}

pub async fn can_referee(pool: &PgPool, user_id: Uuid) -> sqlx::Result<bool> {
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM osu_users WHERE id = $1 AND 'referee' = ANY(roles))",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}
