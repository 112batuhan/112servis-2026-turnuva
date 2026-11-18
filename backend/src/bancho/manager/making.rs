//! Making a room with !mp make and setting it up once BanchoBot answers.

use std::collections::HashSet;

use time::OffsetDateTime;
use tokio::time::Instant;
use uuid::Uuid;

use crate::bancho::{
    connection::{
        bot::Priority,
        irc::{room_channel, BANCHO_BOT},
    },
    room::commands::{player_ref, HEAD_TO_HEAD, TEAM_VS},
    room::RoomState,
};
use crate::{
    db::autoref::LogLevel,
    error::{AppError, BanchoError, GeneralError},
    services,
};

use super::{Making, Manager, Room, MAKE_TIMEOUT};

impl Manager {
    pub(super) fn start_making(&mut self, bot_id: Uuid, mut making: Making) {
        let Some(run) = self.runs.get(&making.match_id) else {
            let _ = making.reply.send(Err(GeneralError::NotFound.into()));
            return;
        };
        let name = run.ctx.room_name.clone();
        making.deadline = Instant::now() + MAKE_TIMEOUT;
        self.bot_send(
            bot_id,
            BANCHO_BOT.into(),
            format!("!mp make {name}"),
            None,
            Priority::High,
        );
        if let Some(bot) = self.bots.get_mut(&bot_id) {
            bot.making = Some(making);
        }
        self.status_changed = true;
    }

    // A room is being made for the match, or waits for a bot.
    pub(super) fn making_for(&self, match_id: Uuid) -> bool {
        self.bots
            .values()
            .filter_map(|b| b.making.as_ref())
            .chain(&self.queued)
            .any(|m| m.match_id == match_id)
    }

    // The rooms waiting for a bot, made as bots come free; one that waited too long is
    // given up on.
    pub(super) fn start_queued(&mut self, now: Instant) {
        for making in std::mem::take(&mut self.queued) {
            if making.deadline <= now {
                let _ = making.reply.send(Err(BanchoError::BotsFull.into()));
            } else {
                self.queued.push_back(making);
            }
        }
        while let Some(making) = self.queued.pop_front() {
            match self.pick_bot(&making.tried, true) {
                Ok(bot) => self.start_making(bot, making),
                Err(_) => {
                    self.queued.push_front(making);
                    break;
                }
            }
        }
    }

    // BanchoBot made the room: it's the match's now, set up and on the site.
    pub(super) async fn room_made(
        &mut self,
        bot_id: Uuid,
        mp_id: i64,
        match_id: Uuid,
        auto: bool,
    ) -> Result<(), AppError> {
        let joined = self
            .bots
            .get_mut(&bot_id)
            .is_some_and(|b| b.joined_unknown.remove(&mp_id));
        self.rooms.insert(
            mp_id,
            Room {
                mp_id,
                match_id,
                bot: Some(bot_id),
                bot_made: true,
                added_at: OffsetDateTime::now_utc(),
                closed: false,
                joined,
                refused: false,
                state: RoomState::default(),
                close_at: None,
                close_ends: None,
                close_warned: false,
                close_told: None,
                unknown: HashSet::new(),
                refs_asked: HashSet::new(),
                tiebreaker_done: None,
                reposition_at: None,
                reposition_tries: 0,
            },
        );
        self.set_up(match_id, mp_id);
        let concluded = self.runs.get(&match_id).is_none_or(|r| r.ctx.concluded());
        let bot_name = self.bot_name(Some(bot_id));
        if auto {
            self.auto_log(
                match_id,
                LogLevel::Info,
                format!(
                    "Oda açıldı: #mp_{mp_id} ({}), oyuncular davet edildi.",
                    bot_name.as_deref().unwrap_or("?")
                ),
            );
        }
        services::matches::link_room(
            &self.app,
            match_id,
            mp_id,
            Some(bot_id),
            true,
            concluded,
            bot_name.filter(|_| auto),
        )
        .await
    }

    // Team mode, score mode and size; locks the slots, then invites everyone.
    pub(super) fn set_up(&self, match_id: Uuid, mp_id: i64) {
        let (Some(run), Some(bot)) = (
            self.runs.get(&match_id),
            self.rooms.get(&mp_id).and_then(|r| r.bot),
        ) else {
            return;
        };
        let ctx = &run.ctx;
        let mode = if ctx.is_bracket() {
            TEAM_VS
        } else {
            HEAD_TO_HEAD
        };
        let size = ctx.roster.len().clamp(2, 16);
        let channel = room_channel(mp_id);
        let mut lines = vec![
            format!("!mp set {mode} {} {size}", ctx.settings.score_mode),
            "!mp lock".to_string(),
        ];
        lines.extend(
            ctx.roster
                .iter()
                .map(|p| format!("!mp invite {}", player_ref(p.osu_id))),
        );
        lines.push("!mp settings".into());
        for line in lines {
            self.bot_send(bot, channel.clone(), line, Some(mp_id), Priority::High);
        }
    }
}
