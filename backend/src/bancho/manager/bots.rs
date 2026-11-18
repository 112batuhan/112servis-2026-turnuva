//! The bot accounts: connecting and disconnecting them, and choosing which one makes
//! or runs a room.

use std::collections::HashSet;

use tokio::time::Instant;
use uuid::Uuid;

use crate::bancho::{
    connection::bot::{self, BotCmd, Priority},
    names::same_name,
    ROOM_LIMIT,
};
use crate::{
    db,
    error::{AppError, BanchoError},
};

use super::{Bot, Manager};

impl Manager {
    pub(super) async fn reload_bots(&mut self) {
        let logins =
            match db::bancho::list_logins(&self.app.db, self.app.bot_secret.as_deref()).await {
                Ok(logins) => logins,
                Err(err) => {
                    tracing::error!(error = ?err, "could not read the bot accounts");
                    return;
                }
            };
        let known: HashSet<Uuid> = logins.iter().map(|l| l.id).collect();
        for id in self.bots.keys().copied().collect::<Vec<_>>() {
            if !known.contains(&id) {
                self.stop_bot(id);
                self.bots.remove(&id);
            }
        }
        for login in logins {
            let id = login.id;
            let bot = self.bots.entry(id).or_insert_with(|| Bot {
                login: login.clone(),
                conn: None,
                online: false,
                online_since: None,
                error: None,
                refused_password: None,
                full_until: None,
                making: None,
                joined_unknown: HashSet::new(),
            });
            let changed = bot.login.irc_password != login.irc_password
                || bot.login.username != login.username;
            if !login.active {
                bot.refused_password = None;
            }
            bot.login = login;
            let running = bot.conn.is_some();
            if !bot.login.active {
                if running {
                    self.stop_bot(id);
                }
            } else if running && changed {
                self.stop_bot(id);
                self.start_bot(id);
            } else if !running {
                self.start_bot(id);
            }
        }
        self.status_changed = true;
    }

    fn start_bot(&mut self, id: Uuid) {
        let conn_id = self.next_conn;
        let Some(bot) = self.bots.get_mut(&id) else {
            return;
        };
        if bot.refused_password.as_deref() == Some(bot.login.irc_password.as_str()) {
            return;
        }
        self.next_conn += 1;
        let conn = bot::spawn(
            conn_id,
            bot.login.username.clone(),
            bot.login.irc_password.clone(),
            self.app.config.bancho_irc_addr.clone(),
            self.events.clone(),
        );
        bot.conn = Some((conn_id, conn));
        bot.error = None;
        self.conns.insert(conn_id, id);
    }

    fn stop_bot(&mut self, id: Uuid) {
        let Some(bot) = self.bots.get_mut(&id) else {
            return;
        };
        if let Some((conn_id, conn)) = bot.conn.take() {
            let _ = conn.tx.send(BotCmd::Stop);
            self.conns.remove(&conn_id);
        }
        bot.online = false;
        bot.online_since = None;
        if let Some(making) = bot.making.take() {
            let _ = making.reply.send(Err(BanchoError::BotTurnedOff.into()));
        }
        self.bot_offline(id);
    }

    // What changes for the rooms when a bot goes offline.
    pub(super) fn bot_offline(&mut self, id: Uuid) {
        for room in self.rooms.values_mut().filter(|r| r.bot == Some(id)) {
            room.joined = false;
        }
        self.mark_bot_rooms(id);
        self.status_changed = true;
    }

    pub(super) fn mark_bot_rooms(&mut self, id: Uuid) {
        let matches: HashSet<Uuid> = self
            .rooms
            .values()
            .filter(|r| r.bot == Some(id))
            .map(|r| r.match_id)
            .collect();
        for m in matches {
            if let Some(run) = self.runs.get_mut(&m) {
                run.dirty = true;
            }
        }
    }

    pub(super) fn bot_send(
        &self,
        bot: Uuid,
        target: String,
        text: String,
        room: Option<i64>,
        priority: Priority,
    ) {
        if let Some((_, conn)) = self.bots.get(&bot).and_then(|b| b.conn.as_ref()) {
            let _ = conn.tx.send(BotCmd::Send {
                target,
                text,
                room,
                priority,
            });
        }
    }

    pub(super) fn bot_raw(&self, bot: Uuid, cmd: BotCmd) {
        if let Some((_, conn)) = self.bots.get(&bot).and_then(|b| b.conn.as_ref()) {
            let _ = conn.tx.send(cmd);
        }
    }

    pub(super) fn bot_online(&self, bot: Option<Uuid>) -> bool {
        bot.and_then(|b| self.bots.get(&b))
            .is_some_and(|b| b.online)
    }

    pub(super) fn is_bot_name(&self, name: &str) -> bool {
        self.bots
            .values()
            .any(|b| same_name(&b.login.username, name))
    }

    // The rooms that count against a bot's limit, a room being made included.
    fn load_of(&self, bot: Uuid) -> usize {
        let open = self
            .rooms
            .values()
            .filter(|r| r.bot == Some(bot) && !r.closed)
            .count();
        open + self
            .bots
            .get(&bot)
            .map_or(0, |b| b.making.is_some() as usize)
    }

    // The least busy bot that can take a room: connected, not full, under the limit.
    // How many more rooms the bots busy making one have room for once it's made: a room
    // asked for meanwhile waits for one of them rather than failing.
    pub(super) fn make_capacity(&self) -> usize {
        let now = Instant::now();
        self.bots
            .values()
            .filter(|b| b.login.active && b.online && b.making.is_some())
            .filter(|b| b.full_until.is_none_or(|t| t <= now))
            .map(|b| ROOM_LIMIT.saturating_sub(self.load_of(b.login.id)))
            .sum()
    }

    pub(super) fn pick_bot(&self, skip: &[Uuid], making: bool) -> Result<Uuid, AppError> {
        let now = Instant::now();
        let online: Vec<&Bot> = self
            .bots
            .values()
            .filter(|b| b.login.active && b.online && !skip.contains(&b.login.id))
            .collect();
        if online.is_empty() {
            return Err(BanchoError::NoBotConnected.into());
        }
        online
            .into_iter()
            .filter(|b| b.full_until.is_none_or(|t| t <= now))
            .filter(|b| !making || b.making.is_none())
            .map(|b| (self.load_of(b.login.id), b))
            .filter(|(load, _)| *load < ROOM_LIMIT)
            .min_by(|(a, x), (b, y)| a.cmp(b).then(x.login.username.cmp(&y.login.username)))
            .map(|(_, b)| b.login.id)
            .ok_or(BanchoError::BotsFull.into())
    }
}
