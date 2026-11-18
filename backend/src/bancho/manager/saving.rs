//! What the manager hands to tasks of their own so it never waits on the database:
//! the rooms' chat, the auto referee's state and log, and the scores, each saved in order.

use tokio::sync::mpsc;
use uuid::Uuid;

use crate::{
    db::{self, stats::QualifierMapRow},
    error::AppError,
    room_scores, services,
    supervise::{self, SharedReceiver},
    AppState,
};

use super::{AutoWrite, ChatWrite, Save, SaveJob};

// Saves room lines in order, then sends each to the pages watching its match.
pub(super) fn spawn_chat_writer(app: AppState) -> mpsc::UnboundedSender<ChatWrite> {
    let (tx, rx) = mpsc::unbounded_channel::<ChatWrite>();
    let rx = supervise::shared(rx);
    supervise::spawn("bancho chat writer", move || {
        write_chat(app.clone(), rx.clone())
    });
    tx
}

// Ends once the manager that feeds it is gone.
async fn write_chat(app: AppState, rx: SharedReceiver<ChatWrite>) {
    let mut rx = rx.lock().await;
    while let Some(w) = rx.recv().await {
        match db::chat::insert_message(&app.db, w.mp_id, &w.from, &w.text, w.action, w.bot, w.at)
            .await
        {
            Ok(message) => app.live.chat(w.mp_id, &message),
            Err(err) => {
                tracing::error!(error = ?err, mp_id = w.mp_id, "could not save a chat line")
            }
        }
    }
}

// Saves the auto referee's state and log lines in order; each log line is sent to the
// referee pages once saved.
pub(super) fn spawn_autoref_writer(app: AppState) -> mpsc::UnboundedSender<AutoWrite> {
    let (tx, rx) = mpsc::unbounded_channel::<AutoWrite>();
    let rx = supervise::shared(rx);
    supervise::spawn("autoref writer", move || {
        write_autoref(app.clone(), rx.clone())
    });
    tx
}

// Ends once the manager that feeds it is gone.
async fn write_autoref(app: AppState, rx: SharedReceiver<AutoWrite>) {
    let mut rx = rx.lock().await;
    while let Some(w) = rx.recv().await {
        let result = match &w {
            AutoWrite::State(id, state) => db::autoref::save_state(&app.db, *id, state)
                .await
                .map_err(AppError::from),
            AutoWrite::Log(id, level, message) => {
                services::autoref::log(&app, *id, *level, message)
                    .await
                    .map(|_| ())
            }
        };
        if let Err(err) = result {
            tracing::error!(error = ?err, "could not save the auto referee's state or log");
        }
    }
}

// Saves scores one after the other, so an older one never lands after a newer one. Each
// save has osu!'s record of the match's rooms read for its accuracy.
pub(super) fn spawn_saver(app: AppState) -> mpsc::UnboundedSender<SaveJob> {
    let (tx, rx) = mpsc::unbounded_channel::<SaveJob>();
    let rx = supervise::shared(rx);
    let scores = room_scores::spawn(app.clone());
    supervise::spawn("bancho score saver", move || {
        save_scores(app.clone(), rx.clone(), scores.clone())
    });
    tx
}

// Ends once the manager that feeds it is gone.
async fn save_scores(
    app: AppState,
    rx: SharedReceiver<SaveJob>,
    scores: mpsc::UnboundedSender<Uuid>,
) {
    let mut rx = rx.lock().await;
    while let Some(job) = rx.recv().await {
        let result = match &job.save {
            Save::Score { team1, team2, maps } => {
                services::bracket::save_score(
                    &app,
                    job.match_id,
                    Some(*team1),
                    Some(*team2),
                    Some(maps),
                )
                .await
            }
            Save::Lobby(rows) => save_lobby(&app, job.match_id, rows).await,
        };
        match &result {
            Ok(()) => {
                let _ = scores.send(job.match_id);
            }
            Err(err) => {
                tracing::error!(error = ?err, match_id = %job.match_id, "could not save a match's score")
            }
        }
        if let Some(reply) = job.reply {
            let _ = reply.send(result);
        }
    }
}

// Saves a qualifier lobby's finished pool maps; the pages showing it (the statistics
// among them) hear of it.
pub(super) async fn save_lobby(
    app: &AppState,
    match_id: Uuid,
    rows: &[QualifierMapRow],
) -> Result<(), AppError> {
    let mut tx = app.db.begin().await?;
    db::stats::replace_qualifier_results(&mut tx, match_id, rows).await?;
    tx.commit().await?;
    app.live.match_changed(match_id);
    Ok(())
}
