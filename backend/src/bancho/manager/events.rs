//! What the bots hear: Bancho's events, each line said in a room, BanchoBot's replies.

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::time::Instant;
use uuid::Uuid;

use crate::bancho::{
    autoref::{self, GRACE},
    connection::{
        bot::{BotCmd, BotEvent},
        irc::{room_channel, room_id},
    },
    room::parse_created,
};
use crate::{db, error::BanchoError};

use super::{
    is_bancho_bot, run_dirty, ChatWrite, Manager, CLOSE_AFTER_CONCLUDED, CLOSE_WARN,
    CLOSE_WARN_TEXT, FULL_BACKOFF,
};

impl Manager {
    pub(super) async fn event(&mut self, event: BotEvent) {
        let conn = match &event {
            BotEvent::Online { conn }
            | BotEvent::Offline { conn, .. }
            | BotEvent::Line { conn, .. }
            | BotEvent::Joined { conn, .. }
            | BotEvent::Parted { conn, .. }
            | BotEvent::Refused { conn, .. } => *conn,
        };
        // An earlier connection's last words.
        let Some(&bot_id) = self.conns.get(&conn) else {
            return;
        };
        match event {
            BotEvent::Online { .. } => {
                if let Some(bot) = self.bots.get_mut(&bot_id) {
                    bot.online = true;
                    bot.online_since = Some(OffsetDateTime::now_utc());
                    bot.error = None;
                }
                let channels: Vec<i64> = self
                    .rooms
                    .values()
                    .filter(|r| r.bot == Some(bot_id) && !r.closed)
                    .map(|r| r.mp_id)
                    .collect();
                for mp_id in channels {
                    self.bot_raw(bot_id, BotCmd::Join(room_channel(mp_id)));
                }
                self.mark_bot_rooms(bot_id);
                self.status_changed = true;
            }
            BotEvent::Offline {
                reason,
                login_failed,
                ..
            } => {
                if let Some(bot) = self.bots.get_mut(&bot_id) {
                    bot.online = false;
                    bot.online_since = None;
                    bot.error = Some(reason);
                    if let Some(making) = bot.making.take() {
                        let _ = making.reply.send(Err(BanchoError::BotDisconnected.into()));
                    }
                    if login_failed {
                        bot.refused_password = Some(bot.login.irc_password.clone());
                        bot.conn = None;
                        self.conns.remove(&conn);
                    }
                }
                self.bot_offline(bot_id);
            }
            BotEvent::Joined { channel, .. } => {
                let Some(mp_id) = room_id(&channel) else {
                    return;
                };
                match self.rooms.get_mut(&mp_id) {
                    Some(room) if room.bot == Some(bot_id) => {
                        room.joined = true;
                        room.refused = false;
                        let match_id = room.match_id;
                        if let Some(run) = self.runs.get_mut(&match_id) {
                            run.dirty = true;
                        }
                    }
                    // Not ours to be in.
                    Some(_) => self.bot_raw(bot_id, BotCmd::Part(channel)),
                    None => {
                        if let Some(bot) = self.bots.get_mut(&bot_id) {
                            bot.joined_unknown.insert(mp_id);
                        }
                    }
                }
                self.status_changed = true;
            }
            BotEvent::Parted { channel, .. } => {
                if let Some(room) = room_id(&channel).and_then(|id| self.rooms.get_mut(&id)) {
                    if room.bot == Some(bot_id) {
                        room.joined = false;
                        let match_id = room.match_id;
                        run_dirty(&mut self.runs, match_id);
                    }
                }
            }
            BotEvent::Refused {
                code,
                subject,
                message,
                ..
            } => {
                tracing::warn!(bot = %bot_id, code, subject, message, "Bancho refused a bot's command");
                let Some(mp_id) = room_id(&subject) else {
                    return;
                };
                let Some(room) = self.rooms.get_mut(&mp_id).filter(|r| r.bot == Some(bot_id))
                else {
                    return;
                };
                // A room the bot made that it can't get into is gone; one made elsewhere
                // hasn't had the bot added yet, unless its match is over.
                if code == "403" {
                    room.joined = false;
                    let over = !self.runs.contains_key(&room.match_id);
                    if room.bot_made || over {
                        self.room_closed(mp_id);
                    } else {
                        room.refused = true;
                        let match_id = room.match_id;
                        run_dirty(&mut self.runs, match_id);
                    }
                    self.status_changed = true;
                }
            }
            BotEvent::Line {
                from,
                target,
                text,
                action,
                at,
                ..
            } => {
                if target.starts_with('#') {
                    self.room_line(bot_id, from, target, text, action, at).await;
                } else if is_bancho_bot(&from) {
                    self.bancho_reply(bot_id, &text).await;
                } else if !action && !self.is_bot_name(&from) {
                    self.private_message(bot_id, &from, &text);
                }
            }
        }
    }

    pub(super) fn room_closed(&mut self, mp_id: i64) {
        let Some(room) = self.rooms.get_mut(&mp_id) else {
            return;
        };
        room.closed = true;
        room.joined = false;
        room.close_at = None;
        let match_id = room.match_id;
        run_dirty(&mut self.runs, match_id);
        let db = self.app.db.clone();
        tokio::spawn(async move {
            if let Err(err) = db::bancho::mark_closed(&db, mp_id).await {
                tracing::error!(error = ?err, mp_id, "could not mark a room closed");
            }
        });
        self.status_changed = true;
    }

    async fn room_line(
        &mut self,
        bot_id: Uuid,
        from: String,
        target: String,
        text: String,
        action: bool,
        at: OffsetDateTime,
    ) {
        let Some(mp_id) = room_id(&target) else {
            return;
        };
        let own = self.is_bot_name(&from);
        let Some(room) = self.rooms.get_mut(&mp_id) else {
            return;
        };
        if room.bot != Some(bot_id) {
            return;
        }
        let _ = self.chat.send(ChatWrite {
            mp_id,
            from: from.clone(),
            text: text.clone(),
            action,
            bot: bot_id,
            at,
        });
        let match_id = room.match_id;
        if action {
            return;
        }
        // A player's line: the auto referee's commands, in the match's latest room.
        if own {
            room.last_own = Some(Instant::now());
            // One of the bot's own lines, as it went out: a countdown line that went out
            // late moves its end back.
            self.closing_said(mp_id, &text, at);
            if self.auto_on(match_id) == Some(mp_id) {
                self.drive(match_id, |auto, _, _| auto.said(&text, at))
                    .await;
            }
            return;
        }
        if !is_bancho_bot(&from) {
            if self.runs.contains_key(&match_id) && self.latest_room(match_id) == Some(mp_id) {
                let enabled = self.auto_on(match_id).is_some();
                self.drive(match_id, |auto, s, out| {
                    auto.player(s, &from, &text, enabled, out)
                })
                .await;
            }
            return;
        }
        let results = room.state.results.len();
        room.state.apply(&text, at);
        let finished = room.state.results.len() != results;
        if text == "Closed the match" {
            self.room_closed(mp_id);
        }
        if self.runs.contains_key(&match_id) {
            self.automate(match_id, mp_id, &text);
            if finished {
                self.count(match_id);
            } else {
                run_dirty(&mut self.runs, match_id);
            }
            if self.auto_on(match_id) == Some(mp_id) {
                self.drive(match_id, |auto, s, out| auto.bancho(s, &text, out))
                    .await;
            }
        }
    }

    // A closing line that went out late moves the room's close back by as long.
    fn closing_said(&mut self, mp_id: i64, text: &str, at: OffsetDateTime) {
        let told = if text == CLOSE_WARN_TEXT {
            CLOSE_WARN
        } else if text == autoref::CLOSING {
            CLOSE_AFTER_CONCLUDED
        } else {
            return;
        };
        let Some(room) = self.rooms.get_mut(&mp_id) else {
            return;
        };
        let ends = at + told;
        if text == autoref::CLOSING {
            room.close_told = Some(ends);
        }
        if room
            .close_ends
            .is_some_and(|e| ends - e > TimeDuration::seconds(1))
        {
            room.close_ends = Some(ends);
            room.close_at = Some(ends + GRACE);
        }
    }

    // BanchoBot answering the bot in private: !mp make's reply, or its refusal.
    async fn bancho_reply(&mut self, bot_id: Uuid, text: &str) {
        if let Some(mp_id) = parse_created(text) {
            let Some(making) = self.bots.get_mut(&bot_id).and_then(|b| b.making.take()) else {
                return;
            };
            self.status_changed = true;
            let result = self
                .room_made(bot_id, mp_id, making.match_id, making.auto)
                .await;
            let _ = making.reply.send(result.map(|()| mp_id));
        } else if text.to_lowercase().contains("cannot create") {
            let Some(bot) = self.bots.get_mut(&bot_id) else {
                return;
            };
            bot.full_until = Some(Instant::now() + FULL_BACKOFF);
            let Some(mut making) = bot.making.take() else {
                return;
            };
            tracing::warn!(bot = %bot_id, text, "Bancho says the bot can't open more rooms");
            making.tried.push(bot_id);
            self.status_changed = true;
            // Another bot may have room for it.
            match self.pick_bot(&making.tried, true) {
                Ok(next) => self.start_making(next, making),
                Err(err) => {
                    let _ = making.reply.send(Err(err));
                }
            }
        }
    }
}
