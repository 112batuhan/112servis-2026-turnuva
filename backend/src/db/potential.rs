use sqlx::{PgConnection, PgExecutor, PgPool};
use time::OffsetDateTime;
use uuid::Uuid;

use super::{
    bracket::BracketLock,
    matches::{MatchKind, RefereeColumns},
};
use crate::{
    discord::jobs::Job,
    potentials::{self, Node, Pairing},
};

// A potential match with its referee and streamer folded in.
#[derive(Debug, sqlx::FromRow)]
pub struct PotentialRow {
    pub id: Uuid,
    pub bracket_match_id: Uuid,
    pub team1_id: Uuid,
    pub team2_id: Uuid,
    pub starts_at: Option<OffsetDateTime>,
    #[sqlx(flatten)]
    pub referee: RefereeColumns,
    pub streamer_id: Option<Uuid>,
    pub streamer_osu_id: Option<i64>,
    pub streamer_username: Option<String>,
    pub streamer_avatar_url: Option<String>,
}

// Every potential match that can still happen, soonest first.
pub async fn list(pool: &PgPool) -> sqlx::Result<Vec<PotentialRow>> {
    sqlx::query_as::<_, PotentialRow>(
        r#"
        SELECT p.match_id AS id, p.bracket_match_id, p.team1_id, p.team2_id, m.starts_at,
               u.id AS referee_id, u.osu_id AS referee_osu_id,
               u.username AS referee_username, u.avatar_url AS referee_avatar_url,
               d.discord_id AS referee_discord_id, d.username AS referee_discord_username,
               s.id AS streamer_id, s.osu_id AS streamer_osu_id,
               s.username AS streamer_username, s.avatar_url AS streamer_avatar_url
        FROM potential_matches p
        JOIN matches m ON m.id = p.match_id
        LEFT JOIN osu_users u ON u.id = m.referee_id
        LEFT JOIN discord_accounts d ON d.osu_user_id = u.id
        LEFT JOIN osu_users s ON s.id = p.streamer_id
        WHERE p.removed_at IS NULL
        ORDER BY m.starts_at NULLS LAST, m.created_at, p.match_id
        "#,
    )
    .fetch_all(pool)
    .await
}

// Every change to the bracket's teams takes this first, before any row lock, so the
// potential matches are worked out by one transaction at a time and two can't create
// the same pairing.
pub async fn lock(conn: &mut PgConnection) -> sqlx::Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(112051)")
        .execute(conn)
        .await?;
    Ok(())
}

// A potential match read like a bracket match, for the routes that serve both: it has
// no room, score or conclusion.
pub async fn find(pool: &PgPool, id: Uuid) -> sqlx::Result<Option<BracketLock>> {
    sqlx::query_as::<_, BracketLock>(
        r#"
        SELECT m.starts_at, m.referee_id, p.team1_id, p.team2_id,
               NULL::int AS team1_score, NULL::int AS team2_score, NULL::smallint AS forfeit_slot,
               p.streamer_id, false AS has_room, m.concluded_at
        FROM potential_matches p
        JOIN matches m ON m.id = p.match_id
        WHERE p.match_id = $1 AND p.removed_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

// A bracket match or a potential match under a row lock, for rescheduling it and
// answering about its time.
pub async fn lock_schedulable(
    conn: &mut PgConnection,
    id: Uuid,
) -> sqlx::Result<Option<BracketLock>> {
    if let Some(m) = super::bracket::lock_match(&mut *conn, id).await? {
        return Ok(Some(m));
    }
    sqlx::query_as::<_, BracketLock>(
        r#"
        SELECT m.starts_at, m.referee_id, p.team1_id, p.team2_id,
               NULL::int AS team1_score, NULL::int AS team2_score, NULL::smallint AS forfeit_slot,
               p.streamer_id, false AS has_room, m.concluded_at
        FROM potential_matches p
        JOIN matches m ON m.id = p.match_id
        WHERE p.match_id = $1 AND p.removed_at IS NULL
        FOR UPDATE OF p, m
        "#,
    )
    .bind(id)
    .fetch_optional(conn)
    .await
}

// Takes an unstreamed potential match. False when someone else got there first.
pub async fn claim_stream(pool: &PgPool, id: Uuid, streamer_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE potential_matches SET streamer_id = $2 \
         WHERE match_id = $1 AND streamer_id IS NULL AND removed_at IS NULL",
    )
    .bind(id)
    .bind(streamer_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

// Drops the streamer only if it's still `streamer_id`.
pub async fn release_stream(pool: &PgPool, id: Uuid, streamer_id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE potential_matches SET streamer_id = NULL WHERE match_id = $1 AND streamer_id = $2",
    )
    .bind(id)
    .bind(streamer_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn set_streamer(pool: &PgPool, id: Uuid, streamer_id: Option<Uuid>) -> sqlx::Result<()> {
    sqlx::query("UPDATE potential_matches SET streamer_id = $2 WHERE match_id = $1")
        .bind(id)
        .bind(streamer_id)
        .execute(pool)
        .await?;
    Ok(())
}

// A bracket match's time, referee and streamer are what its potential matches start
// with. When one of them changes, the potential matches still on the old one follow it;
// the ones their teams or staff have changed since keep theirs. Removed ones follow too,
// so one that comes back is where it would have been. Each returns the potential
// matches that followed.

pub async fn follow_time(
    conn: &mut PgConnection,
    bracket_match_id: Uuid,
    old: Option<OffsetDateTime>,
    new: Option<OffsetDateTime>,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        WITH followed AS (
            UPDATE matches m SET starts_at = $3
            FROM potential_matches p
            WHERE p.match_id = m.id AND p.bracket_match_id = $1
              AND m.starts_at IS NOT DISTINCT FROM $2
            RETURNING m.id, p.removed_at
        )
        SELECT id FROM followed WHERE removed_at IS NULL
        "#,
    )
    .bind(bracket_match_id)
    .bind(old)
    .bind(new)
    .fetch_all(conn)
    .await
}

pub async fn follow_referee(
    ex: impl PgExecutor<'_>,
    bracket_match_id: Uuid,
    old: Option<Uuid>,
    new: Option<Uuid>,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        WITH followed AS (
            UPDATE matches m SET referee_id = $3
            FROM potential_matches p
            WHERE p.match_id = m.id AND p.bracket_match_id = $1
              AND m.referee_id IS NOT DISTINCT FROM $2
            RETURNING m.id, p.removed_at
        )
        SELECT id FROM followed WHERE removed_at IS NULL
        "#,
    )
    .bind(bracket_match_id)
    .bind(old)
    .bind(new)
    .fetch_all(ex)
    .await
}

// A streamer never follows onto a pairing their own team plays in.
pub async fn follow_streamer(
    ex: impl PgExecutor<'_>,
    bracket_match_id: Uuid,
    old: Option<Uuid>,
    new: Option<Uuid>,
) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar(
        r#"
        WITH followed AS (
            UPDATE potential_matches p SET streamer_id = $3
            WHERE p.bracket_match_id = $1 AND p.streamer_id IS NOT DISTINCT FROM $2
              AND NOT EXISTS (
                  SELECT 1 FROM team_members tm
                  WHERE tm.user_id = $3 AND tm.team_id IN (p.team1_id, p.team2_id)
              )
            RETURNING p.match_id, p.removed_at
        )
        SELECT match_id FROM followed WHERE removed_at IS NULL
        "#,
    )
    .bind(bracket_match_id)
    .bind(old)
    .bind(new)
    .fetch_all(ex)
    .await
}

// Sets aside potential matches that can no longer happen, keeping everything on them for
// when they can again. Their channels are archived by the returned syncs.
async fn remove(conn: &mut PgConnection, ids: &[Uuid]) -> sqlx::Result<Vec<Job>> {
    sqlx::query(
        "UPDATE potential_matches SET removed_at = now() WHERE match_id = ANY($1) AND removed_at IS NULL",
    )
    .bind(ids)
    .execute(&mut *conn)
    .await?;
    Ok(ids
        .iter()
        .map(|&match_id| Job::SyncMatch { match_id })
        .collect())
}

// Deletes potential matches for good (their bracket match or a team is going), archiving
// their channels: the returned jobs do that.
async fn delete(conn: &mut PgConnection, ids: &[Uuid]) -> sqlx::Result<Vec<Job>> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let channels: Vec<i64> = sqlx::query_scalar(
        "SELECT channel_id FROM match_discord WHERE match_id = ANY($1) AND channel_id IS NOT NULL",
    )
    .bind(ids)
    .fetch_all(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM matches WHERE id = ANY($1) AND kind = 'potential'")
        .bind(ids)
        .execute(&mut *conn)
        .await?;
    Ok(channels
        .into_iter()
        .map(|channel_id| Job::ArchiveChannel {
            channel_id,
            kind: MatchKind::Potential,
        })
        .collect())
}

// The potential matches of a bracket match that's about to be deleted (removed ones
// too), which would go with it and leave their channels behind.
pub async fn remove_of_match(conn: &mut PgConnection, match_id: Uuid) -> sqlx::Result<Vec<Job>> {
    let ids: Vec<Uuid> =
        sqlx::query_scalar("SELECT match_id FROM potential_matches WHERE bracket_match_id = $1")
            .bind(match_id)
            .fetch_all(&mut *conn)
            .await?;
    delete(conn, &ids).await
}

// The same for a team that's about to be deleted.
pub async fn remove_of_team(conn: &mut PgConnection, team_id: Uuid) -> sqlx::Result<Vec<Job>> {
    let ids: Vec<Uuid> = sqlx::query_scalar(
        "SELECT match_id FROM potential_matches WHERE $1 IN (team1_id, team2_id)",
    )
    .bind(team_id)
    .fetch_all(&mut *conn)
    .await?;
    delete(conn, &ids).await
}

// The teams a bracket match had before a change. A pairing it goes back to being (its
// source reopened) takes the predictions and proposals made for the match back with it.
#[derive(Debug, Clone, Copy)]
pub struct Before {
    pub match_id: Uuid,
    pub team1_id: Option<Uuid>,
    pub team2_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct NodeRow {
    id: Uuid,
    stage_id: Option<Uuid>,
    team1_id: Option<Uuid>,
    team2_id: Option<Uuid>,
    concluded: bool,
    starts_at: Option<OffsetDateTime>,
    referee_id: Option<Uuid>,
    streamer_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct ExistingRow {
    id: Uuid,
    bracket_match_id: Uuid,
    team1_id: Uuid,
    team2_id: Uuid,
    removed: bool,
}

// Brings the potential matches in line with the bracket (crate::potentials says which
// should exist), inside the transaction that changed it and after `lock`:
// * a bracket match that now has both its teams takes over the potential match of that
//   pairing (its time, referee, streamer, open proposals, notifications and pick'em
//   predictions), which is then set aside;
// * a pairing that can no longer happen is set aside, with whatever was predicted and
//   proposed for it;
// * a pairing that can happen again (the match that decided it was reopened) comes
//   back as it was, and one the match had taken over gets back what it took;
// * a new pairing starts with its bracket match's time, referee and streamer.
// Returns the Discord jobs: the set-aside ones' channels are archived, the ones that
// came back are reopened, the new ones get a channel and a match that took one over
// gets its own.
pub async fn sync(conn: &mut PgConnection, before: &[Before]) -> sqlx::Result<Vec<Job>> {
    let nodes = sqlx::query_as::<_, NodeRow>(
        r#"
        SELECT b.match_id AS id, b.stage_id, b.team1_id, b.team2_id,
               m.concluded_at IS NOT NULL AS concluded, m.starts_at, m.referee_id, b.streamer_id
        FROM bracket_matches b
        JOIN matches m ON m.id = b.match_id
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;
    let links = super::bracket::list_links(&mut *conn).await?;
    let existing = sqlx::query_as::<_, ExistingRow>(
        r#"
        SELECT match_id AS id, bracket_match_id, team1_id, team2_id,
               removed_at IS NOT NULL AS removed
        FROM potential_matches
        "#,
    )
    .fetch_all(&mut *conn)
    .await?;

    let pure: Vec<Node> = nodes
        .iter()
        .map(|n| Node {
            id: n.id,
            stage_id: n.stage_id,
            team1_id: n.team1_id,
            team2_id: n.team2_id,
            concluded: n.concluded,
        })
        .collect();
    let wanted = potentials::pairings(&pure, &links);
    let pair_of = |e: &ExistingRow| (e.team1_id, e.team2_id);
    let is_wanted = |e: &ExistingRow| {
        wanted.iter().any(|w| {
            w.match_id == e.bracket_match_id
                && potentials::same_teams((w.team1_id, w.team2_id), pair_of(e))
        })
    };

    let mut jobs = Vec::new();
    // A match with both teams takes over its pairing before the rest are set aside.
    for n in &nodes {
        let (Some(t1), Some(t2)) = (n.team1_id, n.team2_id) else {
            continue;
        };
        if n.concluded {
            continue;
        }
        if let Some(p) = existing.iter().find(|e| {
            !e.removed && e.bracket_match_id == n.id && potentials::same_teams(pair_of(e), (t1, t2))
        }) {
            promote(conn, n.id, p.id, t1, t2).await?;
            jobs.push(Job::SyncMatch { match_id: n.id });
        }
    }

    let gone: Vec<Uuid> = existing
        .iter()
        .filter(|e| !e.removed && !is_wanted(e))
        .map(|e| e.id)
        .collect();
    jobs.extend(remove(conn, &gone).await?);

    for w in &wanted {
        let found = existing.iter().find(|e| {
            e.bracket_match_id == w.match_id
                && potentials::same_teams(pair_of(e), (w.team1_id, w.team2_id))
        });
        let id = match found {
            Some(e) if !e.removed => continue,
            Some(e) => {
                restore(conn, e.id, w).await?;
                e.id
            }
            None => {
                let Some(parent) = nodes.iter().find(|n| n.id == w.match_id) else {
                    continue;
                };
                create(conn, w, parent).await?
            }
        };
        let returning = before.iter().any(|b| {
            b.match_id == w.match_id
                && matches!((b.team1_id, b.team2_id), (Some(a), Some(c))
                    if potentials::same_teams((a, c), (w.team1_id, w.team2_id)))
        });
        if returning {
            adopt(conn, w.match_id, id).await?;
        }
        jobs.push(Job::SyncMatch { match_id: id });
    }
    Ok(jobs)
}

// Brings back a set-aside potential match, its teams in the slots they'd now take.
async fn restore(conn: &mut PgConnection, id: Uuid, w: &Pairing) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE potential_matches SET removed_at = NULL, team1_id = $2, team2_id = $3
        WHERE match_id = $1
        "#,
    )
    .bind(id)
    .bind(w.team1_id)
    .bind(w.team2_id)
    .execute(conn)
    .await?;
    Ok(())
}

async fn create(conn: &mut PgConnection, w: &Pairing, parent: &NodeRow) -> sqlx::Result<Uuid> {
    sqlx::query_scalar(
        r#"
        WITH m AS (
            INSERT INTO matches (kind, starts_at, referee_id) VALUES ('potential', $5, $6)
            RETURNING id
        )
        INSERT INTO potential_matches (match_id, bracket_match_id, team1_id, team2_id, streamer_id)
        SELECT m.id, $1, $2, $3,
               CASE WHEN EXISTS (SELECT 1 FROM team_members tm
                                 WHERE tm.user_id = $4 AND tm.team_id IN ($2, $3))
                    THEN NULL ELSE $4 END
        FROM m
        RETURNING match_id
        "#,
    )
    .bind(w.match_id)
    .bind(w.team1_id)
    .bind(w.team2_id)
    .bind(parent.streamer_id)
    .bind(parent.starts_at)
    .bind(parent.referee_id)
    .fetch_one(conn)
    .await
}

// The bracket match `match_id` takes over potential match `potential_id`, whose pairing
// it turned out to be: the predictions and proposals move to it (and come back with
// `adopt` if it goes back to being undecided).
async fn promote(
    conn: &mut PgConnection,
    match_id: Uuid,
    potential_id: Uuid,
    team1_id: Uuid,
    team2_id: Uuid,
) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        UPDATE matches t SET starts_at = p.starts_at, referee_id = p.referee_id
        FROM matches p
        WHERE t.id = $1 AND p.id = $2
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        r#"
        UPDATE bracket_matches b SET streamer_id = p.streamer_id
        FROM potential_matches p
        WHERE b.match_id = $1 AND p.match_id = $2
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    // What was predicted for this pairing counts for the match; anything left on it
    // from another pairing doesn't.
    sqlx::query("DELETE FROM pickem_predictions WHERE match_id = $1 AND team_id <> ALL($2)")
        .bind(match_id)
        .bind(vec![team1_id, team2_id])
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        r#"
        INSERT INTO pickem_predictions
            (user_id, match_id, team_id, winner_score, loser_score, created_at, updated_at)
        SELECT user_id, $1, team_id, winner_score, loser_score, created_at, updated_at
        FROM pickem_predictions WHERE match_id = $2
        ON CONFLICT (user_id, match_id) DO UPDATE
        SET team_id = EXCLUDED.team_id, winner_score = EXCLUDED.winner_score,
            loser_score = EXCLUDED.loser_score, created_at = EXCLUDED.created_at,
            updated_at = EXCLUDED.updated_at
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM pickem_predictions WHERE match_id = $1")
        .bind(potential_id)
        .execute(&mut *conn)
        .await?;
    // Times still under discussion stay open, unless one is the time it's already at.
    sqlx::query(
        r#"
        INSERT INTO match_reschedules (match_id, team_id, proposed_by, starts_at, created_at)
        SELECT $1, r.team_id, r.proposed_by, r.starts_at, r.created_at
        FROM match_reschedules r
        JOIN matches p ON p.id = r.match_id
        WHERE r.match_id = $2 AND r.starts_at IS DISTINCT FROM p.starts_at
        ON CONFLICT (match_id, team_id, starts_at) DO NOTHING
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("DELETE FROM match_reschedules WHERE match_id = $1")
        .bind(potential_id)
        .execute(&mut *conn)
        .await?;
    // The staff's questions about its time and the players' notices are about the match
    // now, and stay answerable there.
    sqlx::query("UPDATE notifications SET match_id = $1 WHERE match_id = $2")
        .bind(match_id)
        .bind(potential_id)
        .execute(&mut *conn)
        .await?;
    Ok(())
}

// A match that went back to being undecided hands what was predicted and proposed for
// it to the potential match of the pairing it was, which takes the match's time, referee
// and streamer as they are now. What the potential match still had from before (the
// match got its teams some other way than taking it over) gives way to the match's.
async fn adopt(conn: &mut PgConnection, match_id: Uuid, potential_id: Uuid) -> sqlx::Result<()> {
    sqlx::query(
        r#"
        DELETE FROM pickem_predictions o
        USING pickem_predictions n
        WHERE o.match_id = $2 AND n.match_id = $1 AND n.user_id = o.user_id
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE pickem_predictions SET match_id = $2 WHERE match_id = $1")
        .bind(match_id)
        .bind(potential_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        r#"
        DELETE FROM match_reschedules o
        USING match_reschedules n
        WHERE o.match_id = $2 AND n.match_id = $1
          AND n.team_id = o.team_id AND n.starts_at = o.starts_at
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query("UPDATE match_reschedules SET match_id = $2 WHERE match_id = $1")
        .bind(match_id)
        .bind(potential_id)
        .execute(&mut *conn)
        .await?;
    sqlx::query(
        r#"
        UPDATE matches p SET starts_at = t.starts_at, referee_id = t.referee_id
        FROM matches t
        WHERE p.id = $2 AND t.id = $1
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    sqlx::query(
        r#"
        UPDATE potential_matches p
        SET streamer_id = CASE WHEN EXISTS (SELECT 1 FROM team_members tm
                                            WHERE tm.user_id = b.streamer_id
                                              AND tm.team_id IN (p.team1_id, p.team2_id))
                               THEN NULL ELSE b.streamer_id END
        FROM bracket_matches b
        WHERE p.match_id = $2 AND b.match_id = $1
        "#,
    )
    .bind(match_id)
    .bind(potential_id)
    .execute(&mut *conn)
    .await?;
    Ok(())
}
