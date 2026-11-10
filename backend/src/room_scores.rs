//! osu!'s record of the site's rooms, read for the accuracy the chat doesn't say: a few
//! seconds after the bots save a match's maps, and again while a saved score is still
//! without it, two minutes later and then backing off, so a room osu! has no record of is
//! soon left alone. Each saved score takes its accuracy (`match_map_scores.accuracy`),
//! and keeps it when the bots save the maps again.

use std::collections::HashMap;

use tokio::{
    sync::mpsc,
    time::{interval, sleep_until, Duration, Instant, MissedTickBehavior},
};
use uuid::Uuid;

use crate::{
    db::{self, stats::RoomScore},
    error::AppError,
    osu_api,
    supervise::{self, SharedReceiver},
    AppState,
};

// How long after a save the rooms are read: osu! records a map as it ends, but give it
// a moment.
const AFTER_SAVE: Duration = Duration::from_secs(8);
// How often rooms with saved scores still missing their accuracy are looked for.
const SWEEP_EVERY: Duration = Duration::from_secs(60);
// The most rooms read in one sweep.
const SWEEP_ROOMS: i64 = 20;

// Starts the reader. Send it a match whose maps were just saved.
pub fn spawn(app: AppState) -> mpsc::UnboundedSender<Uuid> {
    let (tx, rx) = mpsc::unbounded_channel::<Uuid>();
    let rx = supervise::shared(rx);
    supervise::spawn("room score reader", move || {
        read_rooms(app.clone(), rx.clone())
    });
    tx
}

// Ends once nothing can send it a match any more.
async fn read_rooms(app: AppState, rx: SharedReceiver<Uuid>) {
    let mut rx = rx.lock().await;
    let mut due: HashMap<Uuid, Instant> = HashMap::new();
    let mut sweep = interval(SWEEP_EVERY);
    sweep.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        let next = due.values().min().copied();
        tokio::select! {
            received = rx.recv() => match received {
                // A burst of saves reads the rooms once.
                Some(id) => { due.entry(id).or_insert_with(|| Instant::now() + AFTER_SAVE); }
                None => return,
            },
            _ = sleep_until(next.unwrap_or_else(Instant::now)), if next.is_some() => {
                let now = Instant::now();
                let ready: Vec<Uuid> = due.iter().filter(|(_, at)| **at <= now).map(|(id, _)| *id).collect();
                for id in ready {
                    due.remove(&id);
                    if let Err(err) = read_match(&app, id).await {
                        tracing::warn!(error = ?err, match_id = %id, "could not read osu!'s record of a match's rooms");
                    }
                }
            }
            _ = sweep.tick() => {
                match db::stats::rooms_missing_accuracy(&app.db, SWEEP_ROOMS).await {
                    Ok(rooms) => {
                        for mp_id in rooms {
                            if let Err(err) = read_room(&app, mp_id).await {
                                tracing::warn!(error = ?err, mp_id, "could not read osu!'s record of a room");
                            }
                        }
                    }
                    Err(err) => tracing::error!(error = ?err, "could not look for rooms missing accuracy"),
                }
            }
        }
    }
}

async fn read_match(app: &AppState, match_id: Uuid) -> Result<(), AppError> {
    let Some(m) = db::matches::find_match(&app.db, match_id).await? else {
        return Ok(());
    };
    for mp_id in m.mp_ids {
        read_room(app, mp_id).await?;
    }
    Ok(())
}

// Reads one room's record and gives the match's saved scores their accuracy. The pages
// showing the match hear of any change (the statistics among them). A failed read counts as one that found
// nothing, so it backs off too.
async fn read_room(app: &AppState, mp_id: i64) -> Result<(), AppError> {
    let token =
        osu_api::token::app_token(&app.osu_client, &app.oauth_http, &app.osu_app_token).await?;
    let games = match osu_api::matches::fetch_games(&app.http_client, &token, mp_id).await {
        Ok(games) => games,
        Err(err) => {
            db::stats::apply_room_accuracy(&app.db, mp_id, &[]).await?;
            return match err {
                AppError::OsuApiRequest(e) if osu_api::matches::refused(&e) => Ok(()),
                err => Err(err),
            };
        }
    };
    let scores: Vec<RoomScore> = games
        .iter()
        .filter_map(|g| g.beatmap_id.map(|b| (b, g)))
        .flat_map(|(beatmap_id, g)| {
            g.scores.iter().filter_map(move |s| {
                s.accuracy.map(|accuracy| RoomScore {
                    beatmap_id,
                    osu_id: s.user_id,
                    score: s.score,
                    accuracy,
                })
            })
        })
        .collect();
    let changed = db::stats::apply_room_accuracy(&app.db, mp_id, &scores).await?;
    if changed > 0 {
        if let Some(match_id) = db::stats::match_of_room(&app.db, mp_id).await? {
            app.live.match_changed(match_id);
        }
    }
    Ok(())
}
