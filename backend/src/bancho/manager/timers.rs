//! What the manager does on its clock: opening the rooms the auto referee is due to run,
//! closing finished ones, reading changed matches again and pushing changed rooms.

use time::OffsetDateTime;
use tokio::{sync::oneshot, time::Instant};
use uuid::Uuid;

use crate::bancho::connection::{
    bot::{BotCmd, Priority},
    irc::room_channel,
};
use crate::{
    db::{self, autoref::LogLevel},
    error::BanchoError,
    services,
};

use super::{
    Manager, AUTOREF_TICK, CLOSED_TEXT, CLOSE_WARN, CLOSE_WARN_TEXT, OPEN_BEFORE, OPEN_CHECK,
    OPEN_LATE, OPEN_RETRY,
};

impl Manager {
    // Opens the rooms of the matches in auto mode starting within OPEN_BEFORE.
    async fn open_due_rooms(&mut self) {
        let due = match db::autoref::due_rooms(&self.app.db, OPEN_BEFORE, OPEN_LATE).await {
            Ok(due) => due,
            Err(err) => {
                tracing::error!(error = ?err, "could not read the matches due a room");
                return;
            }
        };
        let now = Instant::now();
        self.open_tried
            .retain(|_, t| now.duration_since(*t) < OPEN_RETRY);
        for match_id in due {
            if self.making_for(match_id) || self.open_tried.contains_key(&match_id) {
                continue;
            }
            self.open_tried.insert(match_id, now);
            let (reply, answer) = oneshot::channel();
            let app = self.app.clone();
            tokio::spawn(async move {
                if let Ok(Err(err)) = answer.await {
                    tracing::warn!(error = ?err, match_id = %match_id, "the auto referee could not open a room");
                    let _ = services::autoref::log(
                        &app,
                        match_id,
                        LogLevel::Error,
                        &format!("Oda açılamadı, 1 dakika sonra tekrar denenecek: {err}"),
                    )
                    .await;
                }
            });
            self.create_room(match_id, reply, true).await;
        }
    }

    pub(super) async fn tick(&mut self) {
        if std::mem::take(&mut self.pending_all) {
            self.pending.clear();
            self.resync().await;
        }
        for id in std::mem::take(&mut self.pending) {
            if self.runs.contains_key(&id) {
                self.reload(id).await;
            } else {
                // A match that got its first room, or was reopened.
                self.pending_all = true;
            }
        }
        let now = Instant::now();

        // !mp make answers that never came.
        for bot in self.bots.values_mut() {
            if let Some(making) = bot.making.take_if(|m| m.deadline <= now) {
                let _ = making
                    .reply
                    .send(Err(BanchoError::BanchobotNoAnswer.into()));
                self.status_changed = true;
            }
        }
        self.start_queued(now);
        self.reposition(now);

        // Rooms whose match is over (or that were replaced) are closed, or left; a
        // finished match's room hears it once more before.
        let wall = OffsetDateTime::now_utc();
        let warn: Vec<(i64, Uuid)> = self
            .rooms
            .values()
            .filter(|r| r.joined && !r.closed && r.bot_made && !r.close_warned)
            .filter(|r| r.close_ends.is_some_and(|e| wall >= e - CLOSE_WARN))
            .filter_map(|r| Some((r.mp_id, r.bot?)))
            .collect();
        for (mp_id, bot) in warn {
            if let Some(room) = self.rooms.get_mut(&mp_id) {
                room.close_warned = true;
            }
            self.bot_send(
                bot,
                room_channel(mp_id),
                CLOSE_WARN_TEXT.into(),
                Some(mp_id),
                Priority::Normal,
            );
        }
        let due: Vec<i64> = self
            .rooms
            .values()
            .filter(|r| r.close_at.is_some_and(|at| at <= wall) && r.joined && !r.closed)
            .map(|r| r.mp_id)
            .collect();
        for mp_id in due {
            let Some(room) = self.rooms.get_mut(&mp_id) else {
                continue;
            };
            room.close_at = None;
            let told = room.close_ends.take().is_some();
            let Some(bot) = room.bot else {
                continue;
            };
            let made = room.bot_made;
            if made {
                if told {
                    self.bot_send(
                        bot,
                        room_channel(mp_id),
                        CLOSED_TEXT.into(),
                        Some(mp_id),
                        Priority::Low,
                    );
                }
                self.bot_send(
                    bot,
                    room_channel(mp_id),
                    "!mp close".into(),
                    Some(mp_id),
                    Priority::Low,
                );
            } else {
                self.bot_raw(bot, BotCmd::Part(room_channel(mp_id)));
                self.room_closed(mp_id);
            }
        }

        // The auto referee's clock, and the rooms it opens before their matches.
        if self
            .last_autoref_tick
            .is_none_or(|t| now.duration_since(t) >= AUTOREF_TICK)
        {
            self.last_autoref_tick = Some(now);
            let running: Vec<Uuid> = self
                .runs
                .keys()
                .copied()
                .filter(|&id| self.auto_on(id).is_some())
                .collect();
            // The others aren't run by it now: whenever one is again, it resumes first.
            for (id, run) in self.runs.iter_mut() {
                if !running.contains(id) {
                    run.auto_live = false;
                }
            }
            for id in running {
                self.drive(id, |auto, s, out| auto.tick(s, out)).await;
            }
        }
        if self
            .last_open_check
            .is_none_or(|t| now.duration_since(t) >= OPEN_CHECK)
        {
            self.last_open_check = Some(now);
            self.open_due_rooms().await;
        }

        // The pages see every change at most a tick late.
        let dirty: Vec<Uuid> = self
            .runs
            .iter_mut()
            .filter_map(|(id, r)| std::mem::take(&mut r.dirty).then_some(*id))
            .collect();
        for id in dirty {
            if let Some(view) = self.view_of(id) {
                self.app.live.room(id, &view);
            }
        }
        if std::mem::take(&mut self.status_changed) {
            self.app.live.bancho_changed();
        }
    }
}
