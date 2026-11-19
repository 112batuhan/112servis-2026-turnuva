//! What the pages ask: making a room, saving the score now, the referee page's room
//! commands, the bots' status.

use tokio::{sync::oneshot, time::Instant};
use uuid::Uuid;

use crate::bancho::{
    connection::{
        bot::{BotCmd, Priority},
        irc::room_channel,
    },
    context,
    names::{normalize_name, same_name},
    room::commands::{
        add_ref_command, kick_command, map_command, match_slots, mods_command, move_commands,
        place_commands, player_ref, remove_ref_command,
    },
    room::RoomSlot,
    tally::{score_message, tally, TallyInput},
    view::{finished_of, RoomAccess, ViewRoom},
    BotRoom, BotState, BotStatus, Request, RoomCommand,
};
use crate::{
    error::{AppError, GeneralError, MatchError, RoomError},
    role::Role,
};

use super::{run_dirty, Making, Manager, Room, Save, SaveJob, QUEUE_TIMEOUT};
use crate::bancho::autoref::WarmupMap;

impl Manager {
    pub(super) async fn request(&mut self, request: Request) {
        match request {
            Request::View { match_id, reply } => {
                let _ = reply.send(self.view_of(match_id));
            }
            Request::CreateRoom { match_id, reply } => {
                self.create_room(match_id, reply, false).await
            }
            Request::AssignBot { reply } => {
                let _ = reply.send(self.pick_bot(&[], false).ok());
            }
            Request::Command {
                match_id,
                command,
                reply,
            } => {
                let _ = reply.send(self.command(match_id, command));
            }
            Request::SaveScore { match_id, reply } => self.save_now(match_id, reply),
            Request::Player {
                match_id,
                user_id,
                action,
                reply,
            } => self.player_action(match_id, user_id, action, reply).await,
            Request::PlayerWarmup {
                match_id,
                user_id,
                beatmap_id,
                map,
                reply,
            } => {
                self.player_warmup(match_id, user_id, beatmap_id, map, reply)
                    .await
            }
            Request::OsuGames {
                match_id,
                at,
                games,
                force,
            } => {
                if let Some(run) = self.runs.get_mut(&match_id) {
                    if run.osu_loading == Some(at) {
                        run.osu_loading = None;
                    }
                    run.osu = Some((at, games));
                    run.force_save |= force;
                    self.count(match_id);
                }
            }
            Request::AutorefOsu {
                match_id,
                confirmed,
            } => {
                self.drive(match_id, |auto, s, out| auto.osu_read(s, confirmed, out))
                    .await;
            }
            Request::AutorefWarmup {
                match_id,
                beatmap_id,
                seconds,
                beatmapset_id,
            } => {
                let map = WarmupMap {
                    seconds,
                    beatmapset_id,
                };
                self.drive(match_id, |auto, s, out| {
                    auto.warmup_described(s, beatmap_id, map, out)
                })
                .await;
            }
            Request::BotsChanged => self.reload_bots().await,
            Request::Resync => self.pending_all = true,
            Request::Status { reply } => {
                let _ = reply.send(self.status());
            }
        }
    }

    pub(super) async fn create_room(
        &mut self,
        match_id: Uuid,
        reply: oneshot::Sender<Result<i64, AppError>>,
        auto: bool,
    ) {
        if !self.runs.contains_key(&match_id) {
            match context::load(&self.app.db, match_id).await {
                Ok(Some(ctx)) if !ctx.concluded() => {
                    let run = self.new_run(match_id, ctx).await;
                    self.runs.insert(match_id, run);
                }
                Ok(Some(_)) => {
                    let _ = reply.send(Err(MatchError::Concluded.into()));
                    return;
                }
                Ok(None) => {
                    let _ = reply.send(Err(GeneralError::NotFound.into()));
                    return;
                }
                Err(err) => {
                    let _ = reply.send(Err(err.into()));
                    return;
                }
            }
        }
        let making = Making {
            match_id,
            reply,
            deadline: Instant::now() + QUEUE_TIMEOUT,
            tried: Vec::new(),
            auto,
        };
        match self.pick_bot(&[], true) {
            Ok(bot) => self.start_making(bot, making),
            // Every bot with room to spare is making a room: this one waits its turn.
            Err(_) if self.make_capacity() > self.queued.len() => self.queued.push_back(making),
            Err(err) => {
                let _ = making.reply.send(Err(err));
            }
        }
    }

    fn save_now(&mut self, match_id: Uuid, reply: oneshot::Sender<Result<(), AppError>>) {
        let Some(run) = self.runs.get(&match_id) else {
            let _ = reply.send(Err(RoomError::MatchNotRun.into()));
            return;
        };
        let Some([a, b]) = run.ctx.sides.clone() else {
            let _ = reply.send(Err(MatchError::NeedsBothTeams.into()));
            return;
        };
        let rooms: Vec<ViewRoom> = self
            .match_rooms(match_id)
            .into_iter()
            .map(|r| ViewRoom {
                mp_id: r.mp_id,
                state: &r.state,
                bot: None,
                access: RoomAccess::In,
            })
            .collect();
        let finished = finished_of(&rooms, run.osu.as_ref().map(|(_, g)| g));
        let t = tally(&TallyInput {
            finished: &finished,
            entries: &run.ctx.entries,
            picks: &run.ctx.picks,
            roster: &run.ctx.roster,
            ignored: &run.ctx.ignored,
            manual: &run.ctx.manual,
            team1_id: a.id,
            team2_id: b.id,
        });
        let _ = self.saver.send(SaveJob {
            match_id,
            save: Save::Score {
                team1: t.team1,
                team2: t.team2,
                maps: t.maps,
            },
            reply: Some(reply),
        });
    }

    fn command(&mut self, match_id: Uuid, command: RoomCommand) -> Result<(), AppError> {
        let run = self.runs.get(&match_id).ok_or(MatchError::NeedsRoom)?;
        let ctx = run.ctx.clone();
        let mp_id = self.latest_room(match_id).ok_or(MatchError::NeedsRoom)?;
        let room = &self.rooms[&mp_id];
        let bot = room.bot.ok_or(RoomError::WithoutBot)?;
        if room.closed {
            return Err(RoomError::Closed.into());
        }
        if !self.bot_online(Some(bot)) {
            return Err(RoomError::BotOffline.into());
        }
        if let RoomCommand::Rejoin = command {
            if let Some(room) = self.rooms.get_mut(&mp_id) {
                room.refused = false;
            }
            self.bot_raw(bot, BotCmd::Join(room_channel(mp_id)));
            run_dirty(&mut self.runs, match_id);
            return Ok(());
        }
        if !room.joined {
            return Err(RoomError::BotNotInRoom.into());
        }
        let state = &room.state;
        let slot_at = |n: i32| {
            state
                .slots
                .iter()
                .find(|s| s.slot == n)
                .cloned()
                .ok_or(RoomError::SlotEmpty)
        };
        let with_id = |slot: RoomSlot| match ctx
            .roster
            .iter()
            .find(|r| Some(r.osu_id) == slot.user_id || same_name(&r.username, &slot.username))
        {
            Some(r) => RoomSlot {
                user_id: Some(r.osu_id),
                ..slot
            },
            None => slot,
        };
        let refresh = || "!mp settings".to_string();
        let lines: Vec<String> = match command {
            RoomCommand::SetUp => {
                self.set_up(match_id, mp_id);
                return Ok(());
            }
            RoomCommand::Settings => vec![refresh()],
            RoomCommand::Invite { osu_id } => {
                if !ctx.roster.iter().any(|r| r.osu_id == osu_id) {
                    return Err(MatchError::PlayerNotInMatch.into());
                }
                vec![format!("!mp invite {}", player_ref(osu_id))]
            }
            RoomCommand::InviteAll => ctx
                .roster
                .iter()
                .map(|r| format!("!mp invite {}", player_ref(r.osu_id)))
                .collect(),
            RoomCommand::Move { slot, to, team } => {
                let mover = with_id(slot_at(slot)?);
                if to.is_some_and(|t| !(1..=16).contains(&t)) {
                    return Err(RoomError::BadSlot.into());
                }
                let mut lines = move_commands(state, &mover, to, team, None);
                if !lines.is_empty() {
                    lines.push(refresh());
                }
                lines
            }
            RoomCommand::PlaceAll => {
                let placements = Self::placements(&ctx);
                let mut lines = place_commands(state, &placements, Some(&match_slots(&placements)));
                if !lines.is_empty() {
                    lines.push(refresh());
                }
                lines
            }
            RoomCommand::Kick { slot } => vec![kick_command(&slot_at(slot)?)],
            RoomCommand::KickOutsiders => {
                if !self.staff_loaded {
                    return Err(RoomError::StaffNotLoaded.into());
                }
                state
                    .slots
                    .iter()
                    .filter(|s| !self.is_member(&ctx, s))
                    .map(kick_command)
                    .collect()
            }
            RoomCommand::Lock { locked } => {
                vec![if locked { "!mp lock" } else { "!mp unlock" }.into()]
            }
            RoomCommand::Timer { seconds } => vec![format!("!mp timer {}", seconds.clamp(1, 600))],
            RoomCommand::AbortTimer => vec!["!mp aborttimer".into()],
            RoomCommand::Start { seconds } => vec![match seconds.min(600) {
                0 => "!mp start".to_string(),
                s => format!("!mp start {s}"),
            }],
            RoomCommand::Abort => vec!["!mp abort".into()],
            RoomCommand::Close => vec!["!mp close".into()],
            RoomCommand::Map { map_id } => {
                let entry = ctx
                    .entries
                    .iter()
                    .find(|e| e.map_id == map_id)
                    .ok_or(MatchError::MapNotInPool)?;
                vec![
                    map_command(entry.beatmap_id, &entry.mode),
                    mods_command(&entry.mods, entry.freemod, ctx.settings.no_fail),
                ]
            }
            RoomCommand::AnnounceScore => {
                let view = self.view_of(match_id).ok_or(GeneralError::NotFound)?;
                let (Some([a, b]), Some(t)) = (&ctx.sides, view.tally) else {
                    return Err(MatchError::NeedsBothTeams.into());
                };
                let next = view.next_turn.and_then(|turn| {
                    [a, b]
                        .into_iter()
                        .find(|t| Some(t.id) == turn.team_id)
                        .map(|t| (turn.kind, t.name.as_str()))
                });
                vec![score_message([&a.name, &b.name], [t.team1, t.team2], next)]
            }
            RoomCommand::AddRef { username } => {
                let member = self
                    .staff
                    .iter()
                    .find(|s| same_name(&s.username, &username))
                    .filter(|s| {
                        [Role::Referee, Role::Streamer, Role::Commentator]
                            .iter()
                            .any(|r| s.roles.contains(r))
                    })
                    .ok_or(RoomError::CantBeReferee)?;
                if self.is_bot_name(&member.username) {
                    return Err(RoomError::BotInBotRoom.into());
                }
                // Streamers and commentators can play; a referee of the room could run it.
                if ctx.roster.iter().any(|p| p.osu_id == member.osu_id) {
                    return Err(RoomError::PlayerCantReferee.into());
                }
                vec![add_ref_command(&member.username)]
            }
            RoomCommand::RemoveRef { username } => {
                if !state.is_ref(&username) {
                    return Err(RoomError::NotReferee.into());
                }
                let nick = normalize_name(&username);
                if let Some(room) = self.rooms.get_mut(&mp_id) {
                    room.refs_asked.remove(&nick);
                }
                vec![remove_ref_command(&username)]
            }
            RoomCommand::Say { text, by } => {
                let text = text.trim();
                if let Some(command) = text.strip_prefix("!mp ") {
                    // These go through the room's settings, which keep bots out of each
                    // other's rooms and rooms on their bots.
                    let verb = command
                        .split_whitespace()
                        .next()
                        .unwrap_or_default()
                        .to_lowercase();
                    if ["make", "addref", "removeref"].contains(&verb.as_str()) {
                        return Err(RoomError::RefsFromSettings.into());
                    }
                    vec![text.to_string()]
                } else if self
                    .bot_name(Some(bot))
                    .is_some_and(|name| same_name(&name, &by))
                {
                    vec![text.to_string()]
                } else {
                    vec![format!("[{by}] {text}")]
                }
            }
            // Handled above.
            RoomCommand::Rejoin => return Ok(()),
        };
        for line in lines {
            self.bot_send(bot, room_channel(mp_id), line, Some(mp_id), Priority::High);
        }
        Ok(())
    }

    fn status(&self) -> Vec<BotStatus> {
        let now = Instant::now();
        self.bots
            .values()
            .map(|b| {
                let id = b.login.id;
                let stats = b
                    .conn
                    .as_ref()
                    .map(|(_, c)| c.stats.lock().unwrap().clone())
                    .unwrap_or_default();
                let state = if !b.login.active {
                    BotState::Off
                } else if b.refused_password.is_some() {
                    BotState::LoginFailed
                } else if !b.online {
                    BotState::Connecting
                } else if b.full_until.is_some_and(|t| t > now) {
                    BotState::Full
                } else {
                    BotState::Online
                };
                let mut rooms: Vec<&Room> = self
                    .rooms
                    .values()
                    .filter(|r| r.bot == Some(id) && !r.closed)
                    .collect();
                rooms.sort_by_key(|r| (r.added_at, r.mp_id));
                BotStatus {
                    id,
                    state,
                    error: b
                        .error
                        .clone()
                        .filter(|_| state != BotState::Online && state != BotState::Off),
                    online_since: b.online_since,
                    rooms: rooms
                        .into_iter()
                        .map(|r| BotRoom {
                            mp_id: r.mp_id,
                            match_id: r.match_id,
                            label: self.runs.get(&r.match_id).map(|run| run.ctx.label.clone()),
                            made: r.bot_made,
                            access: self.access(r),
                        })
                        .collect(),
                    queued: stats.queued,
                    recent: stats.recent,
                }
            })
            .collect()
    }
}
