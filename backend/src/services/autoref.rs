//! What the auto referee (bancho/autoref.rs) does to the site's data beyond the room:
//! telling the staff (a player's -rapor or -durdur, or that it handed a match over),
//! switching a match to manual, and settling the teams that didn't show up. The
//! referee page's switch (handlers/referee.rs) goes through `set_enabled` too.

use uuid::Uuid;

use crate::{
    db::{
        self,
        autoref::{LogEntry, LogLevel},
        notifications::NotificationKind,
    },
    discord::jobs::{Alert, Job},
    error::AppError,
    services, AppState,
};

// Writes a line in the match's auto referee log and sends it to its referee pages.
pub async fn log(
    state: &AppState,
    match_id: Uuid,
    level: LogLevel,
    message: &str,
) -> Result<LogEntry, AppError> {
    let entry = db::autoref::add_log(&state.db, match_id, level, message).await?;
    state.live.autoref_log(match_id, &entry);
    Ok(entry)
}

// Switches the match between the auto referee and manual control; the room's bot
// hears it at once. `by`: who did it, for the log.
pub async fn set_enabled(
    state: &AppState,
    match_id: Uuid,
    enabled: bool,
    by: &str,
) -> Result<(), AppError> {
    db::autoref::set_enabled(&state.db, match_id, enabled).await?;
    let message = if enabled {
        format!("{by} otomatik hakemi açtı.")
    } else {
        format!("{by} manuel kontrole geçti, otomatik hakem durdu.")
    };
    log(state, match_id, LogLevel::Info, &message).await?;
    state.live.referee_changed(match_id);
    Ok(())
}

// A player's -rapor or -durdur, or the auto referee stopping: every host and referee
// gets a notification, and the referee channel a ping.
pub async fn alert(
    state: &AppState,
    match_id: Uuid,
    alert: Alert,
    player: Option<(Uuid, Uuid)>,
    text: Option<&str>,
) -> Result<(), AppError> {
    let kind = match alert {
        Alert::Report => NotificationKind::PlayerReport,
        Alert::Stop => NotificationKind::EmergencyStop,
        Alert::Handover => NotificationKind::AutorefStopped,
    };
    let mut tx = state.db.begin().await?;
    let notified = db::notifications::notify_match_staff(
        &mut tx,
        kind,
        match_id,
        player.map(|(_, team)| team),
        player.map(|(user, _)| user),
        text,
    )
    .await?;
    state
        .discord
        .enqueue(
            &mut *tx,
            &[Job::AutorefAlert {
                match_id,
                alert,
                user_id: player.map(|(user, _)| user),
                text: text.map(str::to_string),
            }],
        )
        .await?;
    tx.commit().await?;
    state.live.notifications_changed(notified);
    Ok(())
}

// A player's -durdur: a match the auto referee runs goes to manual (a referee switches
// it back, and it continues where it stopped), and the staff hear it either way.
pub async fn emergency_stop(
    state: &AppState,
    match_id: Uuid,
    player: (Uuid, Uuid),
    username: &str,
    auto: bool,
) -> Result<(), AppError> {
    let message = if auto {
        db::autoref::set_enabled(&state.db, match_id, false).await?;
        format!("{username} -durdur yazdı: manuel kontrole geçildi.")
    } else {
        format!("{username} -durdur yazdı (maç zaten manuel kontrolde).")
    };
    log(state, match_id, LogLevel::Warn, &message).await?;
    state.live.referee_changed(match_id);
    alert(state, match_id, Alert::Stop, Some(player), None).await
}

// The auto referee can't go on: the match goes to manual, and the staff hear why.
pub async fn handover(state: &AppState, match_id: Uuid, reason: &str) -> Result<(), AppError> {
    db::autoref::set_enabled(&state.db, match_id, false).await?;
    log(
        state,
        match_id,
        LogLevel::Error,
        &format!("Manuel kontrole geçildi: {reason}"),
    )
    .await?;
    state.live.referee_changed(match_id);
    alert(state, match_id, Alert::Handover, None, Some(reason)).await
}

// A bracket match forfeited by the team in `slot` (1 or 2), which didn't show up. Both
// teams' players hear it; the absent team's also in a direct message.
pub async fn forfeit_no_show(state: &AppState, match_id: Uuid, slot: i16) -> Result<(), AppError> {
    services::bracket::conclude(state, match_id, Some(slot)).await?;
    state.live.bracket_changed();
    let Some(m) = db::bracket::find_match(&state.db, match_id).await? else {
        return Ok(());
    };
    let (Some(team1), Some(team2)) = (m.team1_id, m.team2_id) else {
        return Ok(());
    };
    let absent = if slot == 1 { team1 } else { team2 };
    let mut tx = state.db.begin().await?;
    let notified = db::notifications::notify_no_show(
        &mut tx,
        NotificationKind::NoShowForfeit,
        match_id,
        absent,
        &[team1, team2],
    )
    .await?;
    state
        .discord
        .enqueue(
            &mut *tx,
            &[Job::NoShow {
                match_id,
                team_id: absent,
                forfeit: true,
            }],
        )
        .await?;
    tx.commit().await?;
    state.live.notifications_changed(notified);
    Ok(())
}

// Teams that didn't show up to their lobby are taken out of it, free to sign up for
// another; their players hear it on the site and in a direct message.
pub async fn lobby_no_show(
    state: &AppState,
    match_id: Uuid,
    teams: &[Uuid],
) -> Result<(), AppError> {
    for &team_id in teams {
        services::matches::remove_lobby_team(state, team_id).await?;
        let mut tx = state.db.begin().await?;
        let notified = db::notifications::notify_no_show(
            &mut tx,
            NotificationKind::LobbyNoShow,
            match_id,
            team_id,
            &[team_id],
        )
        .await?;
        state
            .discord
            .enqueue(
                &mut *tx,
                &[Job::NoShow {
                    match_id,
                    team_id,
                    forfeit: false,
                }],
            )
            .await?;
        tx.commit().await?;
        state.live.notifications_changed(notified);
    }
    Ok(())
}
