use serde::{Deserialize, Serialize};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::bracket::Side;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "notification_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum NotificationKind {
    MatchRescheduled,
    ConfirmTime,
    StaffDeclined,
    RescheduleProposed,
    RankOverLimit,
    DiscordLeft,
    RescheduleAccepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "match_staff_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum StaffRole {
    Referee,
    Streamer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, sqlx::Type)]
#[sqlx(type_name = "notification_response", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Response {
    Confirmed,
    Declined,
}

// A notification with the match or team it's about.
#[derive(Debug, sqlx::FromRow)]
pub struct NotificationRow {
    pub id: Uuid,
    pub kind: NotificationKind,
    pub match_id: Option<Uuid>,
    // The bracket match it is, or for a potential match the one it's a pairing of.
    pub bracket_match_id: Option<Uuid>,
    pub potential: bool,
    pub side: Option<Side>,
    pub number: Option<i32>,
    pub team1_name: Option<String>,
    pub team2_name: Option<String>,
    pub actor_username: Option<String>,
    pub actor_is_host: bool,
    pub staff_role: Option<StaffRole>,
    pub old_starts_at: Option<OffsetDateTime>,
    pub new_starts_at: Option<OffsetDateTime>,
    pub response: Option<Response>,
    pub responded_at: Option<OffsetDateTime>,
    pub read_at: Option<OffsetDateTime>,
    pub created_at: OffsetDateTime,
    pub team_id: Option<Uuid>,
    pub team_name: Option<String>,
    pub old_rank: Option<i64>,
    pub new_rank: Option<i64>,
    pub team_value: Option<f64>,
    // reschedule_proposed: how many of the opponent's proposals are still open.
    pub pending_proposals: i64,
    pub actionable: bool,
}

// An unanswered confirm_time is actionable while its recipient still holds that slot
// and the match is unconcluded and still at the time it asks about. The proposals a
// reschedule_proposed counts are the ones still open from the recipient's opponent.
pub async fn list(pool: &PgPool, user_id: Uuid, limit: i64) -> sqlx::Result<Vec<NotificationRow>> {
    sqlx::query_as::<_, NotificationRow>(
        r#"
        SELECT n.id, n.kind, n.match_id, b.bracket_match_id,
               COALESCE(b.potential, false) AS potential,
               b.side, b.number,
               t1.name AS team1_name, t2.name AS team2_name, a.username AS actor_username,
               COALESCE('host' = ANY(a.roles), false) AS actor_is_host, n.staff_role, n.old_starts_at, n.new_starts_at, n.response, n.responded_at,
               n.read_at, n.created_at,
               n.team_id, tn.name AS team_name, n.old_rank, n.new_rank, n.team_value,
               (SELECT count(*) FROM match_reschedules r
                WHERE r.match_id = n.match_id AND r.team_id <> n.team_id
                  AND n.team_id IN (b.team1_id, b.team2_id)
                  AND r.team_id IN (b.team1_id, b.team2_id)) AS pending_proposals,
               COALESCE(
                   n.kind = 'confirm_time' AND n.response IS NULL
                   AND m.concluded_at IS NULL AND m.starts_at = n.new_starts_at
                   AND CASE n.staff_role WHEN 'referee' THEN m.referee_id = n.user_id
                                         ELSE m.streamer_id = n.user_id END,
                   false
               ) AS actionable
        FROM notifications n
        LEFT JOIN matches m ON m.id = n.match_id
        LEFT JOIN (
            SELECT match_id, match_id AS bracket_match_id, false AS potential, side, number,
                   team1_id, team2_id
            FROM bracket_matches
            UNION ALL
            SELECT p.match_id, p.bracket_match_id, true, pb.side, pb.number,
                   p.team1_id, p.team2_id
            FROM potential_matches p
            JOIN bracket_matches pb ON pb.match_id = p.bracket_match_id
        ) AS b ON b.match_id = n.match_id
        LEFT JOIN teams t1 ON t1.id = b.team1_id
        LEFT JOIN teams t2 ON t2.id = b.team2_id
        LEFT JOIN teams tn ON tn.id = n.team_id
        LEFT JOIN osu_users a ON a.id = n.actor_id
        WHERE n.user_id = $1
          -- A set-aside potential match's wait until it comes back.
          AND NOT EXISTS (SELECT 1 FROM potential_matches x
                          WHERE x.match_id = n.match_id AND x.removed_at IS NOT NULL)
        ORDER BY n.created_at DESC, n.id
        LIMIT $2
        "#,
    )
    .bind(user_id)
    .bind(limit)
    .fetch_all(pool)
    .await
}

// What the bell counts: unread notifications plus confirmations still waiting for an
// answer, read or not.
pub async fn attention_count(pool: &PgPool, user_id: Uuid) -> sqlx::Result<i64> {
    sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM notifications n
        LEFT JOIN matches m ON m.id = n.match_id
        LEFT JOIN (
            SELECT match_id, match_id AS bracket_match_id, false AS potential, side, number,
                   team1_id, team2_id
            FROM bracket_matches
            UNION ALL
            SELECT p.match_id, p.bracket_match_id, true, pb.side, pb.number,
                   p.team1_id, p.team2_id
            FROM potential_matches p
            JOIN bracket_matches pb ON pb.match_id = p.bracket_match_id
        ) AS b ON b.match_id = n.match_id
        WHERE n.user_id = $1
          AND NOT EXISTS (SELECT 1 FROM potential_matches x
                          WHERE x.match_id = n.match_id AND x.removed_at IS NOT NULL)
          AND (n.read_at IS NULL
               OR COALESCE(
                      n.kind = 'confirm_time' AND n.response IS NULL
                      AND m.concluded_at IS NULL AND m.starts_at = n.new_starts_at
                      AND CASE n.staff_role WHEN 'referee' THEN m.referee_id = n.user_id
                                            ELSE m.streamer_id = n.user_id END,
                      false
                  ))
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn mark_all_read(pool: &PgPool, user_id: Uuid) -> sqlx::Result<()> {
    sqlx::query("UPDATE notifications SET read_at = now() WHERE user_id = $1 AND read_at IS NULL")
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(())
}

// After a match moves (its teams agreed on a time, or a host changed it): its referee
// and streamer are asked to confirm the new time, replacing any question about an
// earlier move they haven't answered. Whoever moved it isn't asked.
pub async fn ask_confirm_time(
    conn: &mut PgConnection,
    match_id: Uuid,
    actor_id: Uuid,
    referee_id: Option<Uuid>,
    streamer_id: Option<Uuid>,
    old_starts_at: Option<OffsetDateTime>,
    new_starts_at: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query(
        "DELETE FROM notifications WHERE match_id = $1 AND kind = 'confirm_time' AND response IS NULL",
    )
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO notifications
            (user_id, kind, match_id, actor_id, staff_role, old_starts_at, new_starts_at)
        SELECT s.user_id, 'confirm_time'::notification_kind, $1, $2, s.role, $5, $6
        FROM (VALUES ($3::uuid, 'referee'::match_staff_role),
                     ($4::uuid, 'streamer'::match_staff_role)) AS s (user_id, role)
        WHERE s.user_id IS NOT NULL AND s.user_id <> $2
        "#,
    )
    .bind(match_id)
    .bind(actor_id)
    .bind(referee_id)
    .bind(streamer_id)
    .bind(old_starts_at)
    .bind(new_starts_at)
    .execute(conn)
    .await?;
    Ok(())
}

// After a match's teams move it: its referee and streamer are asked to confirm the new
// time, and the hosts who aren't one of them are told.
pub async fn notify_rescheduled(
    conn: &mut PgConnection,
    match_id: Uuid,
    actor_id: Uuid,
    referee_id: Option<Uuid>,
    streamer_id: Option<Uuid>,
    old_starts_at: Option<OffsetDateTime>,
    new_starts_at: OffsetDateTime,
) -> sqlx::Result<()> {
    ask_confirm_time(
        &mut *conn,
        match_id,
        actor_id,
        referee_id,
        streamer_id,
        old_starts_at,
        new_starts_at,
    )
    .await?;
    sqlx::query(
        r#"
        INSERT INTO notifications
            (user_id, kind, match_id, actor_id, staff_role, old_starts_at, new_starts_at)
        SELECT u.id, 'match_rescheduled'::notification_kind, $1, $2, NULL::match_staff_role, $5, $6
        FROM osu_users u
        WHERE 'host' = ANY(u.roles)
          AND u.id IS DISTINCT FROM $3 AND u.id IS DISTINCT FROM $4
        "#,
    )
    .bind(match_id)
    .bind(actor_id)
    .bind(referee_id)
    .bind(streamer_id)
    .bind(old_starts_at)
    .bind(new_starts_at)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn confirm_match_id(
    pool: &PgPool,
    id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<Option<Uuid>> {
    sqlx::query_scalar(
        "SELECT match_id FROM notifications WHERE id = $1 AND user_id = $2 AND kind = 'confirm_time'",
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

#[derive(Debug, sqlx::FromRow)]
pub struct ConfirmLock {
    pub match_id: Uuid,
    pub staff_role: StaffRole,
    pub old_starts_at: Option<OffsetDateTime>,
    pub new_starts_at: OffsetDateTime,
    pub response: Option<Response>,
}

pub async fn lock_confirm(
    conn: &mut PgConnection,
    id: Uuid,
    user_id: Uuid,
) -> sqlx::Result<Option<ConfirmLock>> {
    sqlx::query_as::<_, ConfirmLock>(
        r#"
        SELECT match_id, staff_role, old_starts_at, new_starts_at, response
        FROM notifications
        WHERE id = $1 AND user_id = $2 AND kind = 'confirm_time'
        FOR UPDATE
        "#,
    )
    .bind(id)
    .bind(user_id)
    .fetch_optional(conn)
    .await
}

pub async fn respond(conn: &mut PgConnection, id: Uuid, response: Response) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE notifications SET response = $2, responded_at = now(), read_at = COALESCE(read_at, now()) WHERE id = $1",
    )
    .bind(id)
    .bind(response)
    .execute(conn)
    .await?;
    Ok(())
}

// Takes whoever holds the slot off the match (a bracket or a potential match).
pub async fn clear_staff(
    conn: &mut PgConnection,
    match_id: Uuid,
    role: StaffRole,
) -> sqlx::Result<()> {
    let queries: &[&str] = match role {
        StaffRole::Referee => &["UPDATE matches SET referee_id = NULL WHERE id = $1"],
        StaffRole::Streamer => &["UPDATE matches SET streamer_id = NULL WHERE id = $1"],
    };
    for query in queries {
        sqlx::query(query)
            .bind(match_id)
            .execute(&mut *conn)
            .await?;
    }
    Ok(())
}

// Tells everyone who could take a declined slot, apart from the one who declined: the
// referees for a referee, the streamers who don't play in the match for a streamer.
// Hosts only if they hold that role themselves.
pub async fn notify_declined(
    conn: &mut PgConnection,
    match_id: Uuid,
    actor_id: Uuid,
    role: StaffRole,
    old_starts_at: Option<OffsetDateTime>,
    new_starts_at: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO notifications
            (user_id, kind, match_id, actor_id, staff_role, old_starts_at, new_starts_at)
        SELECT u.id, 'staff_declined', $1, $2, $3, $4, $5
        FROM osu_users u
        WHERE u.id <> $2
          AND CASE $3
                  WHEN 'referee'::match_staff_role THEN 'referee' = ANY(u.roles)
                  ELSE 'streamer' = ANY(u.roles) AND NOT EXISTS (
                      SELECT 1 FROM team_members tm
                      JOIN (
                          SELECT match_id, team1_id, team2_id FROM bracket_matches
                          UNION ALL
                          SELECT match_id, team1_id, team2_id FROM potential_matches
                      ) AS b ON tm.team_id IN (b.team1_id, b.team2_id)
                      WHERE b.match_id = $1 AND tm.user_id = u.id
                  )
              END
        "#,
    )
    .bind(match_id)
    .bind(actor_id)
    .bind(role)
    .bind(old_starts_at)
    .bind(new_starts_at)
    .execute(conn)
    .await?;
    Ok(())
}

// Tells both players of `team_id` that their opponent proposed a time for the match,
// unless they still have an unread notice about it: one notice covers every proposal
// made before they read it.
pub async fn notify_proposed(
    conn: &mut PgConnection,
    match_id: Uuid,
    team_id: Uuid,
    actor_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, kind, match_id, team_id, actor_id)
        SELECT tm.user_id, 'reschedule_proposed', $1, $2, $3
        FROM team_members tm
        WHERE tm.team_id = $2
          AND NOT EXISTS (
              SELECT 1 FROM notifications n
              WHERE n.user_id = tm.user_id AND n.kind = 'reschedule_proposed'
                AND n.match_id = $1 AND n.read_at IS NULL
          )
        "#,
    )
    .bind(match_id)
    .bind(team_id)
    .bind(actor_id)
    .execute(conn)
    .await?;
    Ok(())
}

// After a match's teams move it: every player of both teams but the one who accepted
// is told, replacing their unread notice of an earlier move of the match.
pub async fn notify_players_rescheduled(
    conn: &mut PgConnection,
    match_id: Uuid,
    team_ids: [Uuid; 2],
    actor_id: Uuid,
    old_starts_at: Option<OffsetDateTime>,
    new_starts_at: OffsetDateTime,
) -> sqlx::Result<()> {
    sqlx::query(
        "DELETE FROM notifications WHERE match_id = $1 AND kind = 'reschedule_accepted' AND read_at IS NULL",
    )
    .bind(match_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        r#"
        INSERT INTO notifications
            (user_id, kind, match_id, team_id, actor_id, old_starts_at, new_starts_at)
        SELECT tm.user_id, 'reschedule_accepted'::notification_kind, $1, tm.team_id, $4, $5, $6
        FROM team_members tm
        WHERE tm.team_id IN ($2, $3) AND tm.user_id <> $4
        "#,
    )
    .bind(match_id)
    .bind(team_ids[0])
    .bind(team_ids[1])
    .bind(actor_id)
    .bind(old_starts_at)
    .bind(new_starts_at)
    .execute(conn)
    .await?;
    Ok(())
}

// Tells a team's players and the hosts that `actor_id`'s rank change took the team
// over the limit.
pub async fn notify_over_limit(
    pool: &PgPool,
    team_id: Uuid,
    actor_id: Uuid,
    old_rank: Option<i64>,
    new_rank: Option<i64>,
    team_value: f64,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO notifications (user_id, kind, team_id, actor_id, old_rank, new_rank, team_value)
        SELECT r.id, 'rank_over_limit', $1, $2, $3, $4, $5
        FROM (
            SELECT user_id AS id FROM team_members WHERE team_id = $1
            UNION
            SELECT id FROM osu_users WHERE 'host' = ANY(roles)
        ) AS r
        "#,
    )
    .bind(team_id)
    .bind(actor_id)
    .bind(old_rank)
    .bind(new_rank)
    .bind(team_value)
    .execute(pool)
    .await?;
    Ok(())
}
