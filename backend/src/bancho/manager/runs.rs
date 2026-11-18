//! The rooms and the matches running in them: loading a match, counting its score and
//! building the view every page reads.

use std::collections::{HashMap, HashSet};

use tokio::time::Instant;
use uuid::Uuid;

use crate::bancho::{
    autoref::GRACE,
    connection::{
        bot::{BotCmd, Priority},
        irc::room_channel,
    },
    context::{self, MatchContext},
    room::commands::{map_command, mods_command},
    room::read_lobby,
    tally::{
        lobby_key, lobby_rows, score_key, score_message, standing_of, tally, Standing, TallyInput,
    },
    view::{self, finished_of, LiveRoom, RoomAccess, ViewRoom},
    Request,
};
use crate::db::{self, matches::MatchKind};

use super::read_osu_games;
use super::{
    is_bancho_bot, run_dirty, Manager, Room, Run, Save, SaveJob, CLOSE_AFTER_CONCLUDED,
    CLOSE_AFTER_REPLACED, REPOSITION_SETTLE,
};

impl Manager {
    pub(super) async fn resync(&mut self) {
        match db::user::list_staff(&self.app.db).await {
            Ok(staff) => {
                self.staff = staff;
                self.staff_loaded = true;
            }
            Err(err) => tracing::error!(error = ?err, "could not read the staff"),
        }
        let tracked = match db::bancho::list_tracked_rooms(&self.app.db).await {
            Ok(rooms) => rooms,
            Err(err) => {
                tracing::error!(error = ?err, "could not read the rooms");
                return;
            }
        };
        let ids: HashSet<i64> = tracked.iter().map(|r| r.mp_id).collect();
        for mp_id in self.rooms.keys().copied().collect::<Vec<_>>() {
            if !ids.contains(&mp_id) {
                self.forget_room(mp_id);
            }
        }
        for t in &tracked {
            if let Some(room) = self.rooms.get_mut(&t.mp_id) {
                room.match_id = t.match_id;
                room.added_at = t.added_at;
                if room.bot != t.bot_id {
                    room.bot = t.bot_id;
                    room.joined = false;
                }
                room.closed |= t.closed_at.is_some();
                continue;
            }
            let lines = db::chat::list_messages(&self.app.db, t.mp_id)
                .await
                .unwrap_or_else(|err| {
                    tracing::error!(error = ?err, mp_id = t.mp_id, "could not read a room's chat");
                    Vec::new()
                });
            let state = read_lobby(
                lines
                    .iter()
                    .filter(|l| is_bancho_bot(&l.sender) && !l.action)
                    .map(|l| (l.text.as_str(), l.sent_at)),
            );
            let closed = t.closed_at.is_some() || state.closed;
            self.rooms.insert(
                t.mp_id,
                Room {
                    mp_id: t.mp_id,
                    match_id: t.match_id,
                    bot: t.bot_id,
                    bot_made: t.bot_made,
                    added_at: t.added_at,
                    closed,
                    joined: false,
                    refused: false,
                    state,
                    close_at: None,
                    close_ends: None,
                    close_warned: false,
                    close_told: None,
                    unknown: HashSet::new(),
                    refs_asked: HashSet::new(),
                    tiebreaker_done: None,
                    reposition_at: Some(Instant::now() + REPOSITION_SETTLE),
                    reposition_tries: 0,
                    last_own: None,
                },
            );
            // A bot that isn't connected yet joins its rooms once it is.
            if let (Some(bot), false) = (t.bot_id, closed) {
                if self.bot_online(Some(bot)) {
                    self.bot_raw(bot, BotCmd::Join(room_channel(t.mp_id)));
                }
            }
        }
        // A concluded match's room, and one replaced by a newer room, is closed a while after.
        let mut by_match: HashMap<Uuid, Vec<&db::bancho::RoomOwner>> = HashMap::new();
        for t in &tracked {
            by_match.entry(t.match_id).or_default().push(t);
        }
        for rooms in by_match.values() {
            for (i, t) in rooms.iter().enumerate() {
                let Some(room) = self.rooms.get_mut(&t.mp_id) else {
                    continue;
                };
                let open = !room.closed && room.bot.is_some();
                match (t.concluded_at, rooms.get(i + 1)) {
                    (Some(at), _) => {
                        // The latest end the room was told, whatever came first.
                        let ends = [
                            Some(at + CLOSE_AFTER_CONCLUDED),
                            room.close_ends,
                            room.close_told,
                        ]
                        .into_iter()
                        .flatten()
                        .max()
                        .filter(|_| open);
                        room.close_ends = ends;
                        room.close_at = ends.map(|e| e + GRACE);
                    }
                    (None, Some(next)) => {
                        room.close_ends = None;
                        room.close_at = open.then_some(next.added_at + CLOSE_AFTER_REPLACED);
                    }
                    // Not over (or reopened).
                    (None, None) => {
                        room.close_ends = None;
                        room.close_warned = false;
                        room.close_at = None;
                    }
                }
            }
        }

        let running: Vec<Uuid> = match db::bancho::list_running_matches(&self.app.db).await {
            Ok(ids) => ids,
            Err(err) => {
                tracing::error!(error = ?err, "could not read the running matches");
                return;
            }
        };
        let running_set: HashSet<Uuid> = running.iter().copied().collect();
        // A match whose room is being made (or waits for a bot) has none yet, but keeps its
        // run: the room is put on it once BanchoBot answers.
        for id in self.runs.keys().copied().collect::<Vec<_>>() {
            if !running_set.contains(&id) && !self.making_for(id) {
                self.end_run(id);
            }
        }
        for id in running {
            self.reload(id).await;
        }
        self.status_changed = true;
    }

    // A room the site no longer has (taken off its match, or the match deleted): a room
    // the bot made is closed, one it was added to is left.
    fn forget_room(&mut self, mp_id: i64) {
        let Some(room) = self.rooms.remove(&mp_id) else {
            return;
        };
        let Some(bot) = room.bot.filter(|_| !room.closed) else {
            return;
        };
        if room.bot_made {
            self.bot_send(
                bot,
                room_channel(mp_id),
                "!mp close".into(),
                Some(mp_id),
                Priority::Normal,
            );
        }
        self.bot_raw(bot, BotCmd::Part(room_channel(mp_id)));
        self.status_changed = true;
    }

    // The match is over or gone: nothing is run for it any more.
    fn end_run(&mut self, id: Uuid) {
        self.runs.remove(&id);
    }

    // Reads the match from the database again, and counts it anew.
    pub(super) async fn reload(&mut self, id: Uuid) {
        let ctx = match context::load(&self.app.db, id).await {
            Ok(Some(ctx)) => ctx,
            Ok(None) => return self.end_run(id),
            Err(err) => {
                tracing::error!(error = ?err, match_id = %id, "could not read a match");
                return;
            }
        };
        if ctx.concluded() {
            self.end_run(id);
            return;
        }
        let synced_at = ctx.settings.osu_synced_at;
        let fresh = !self.runs.contains_key(&id);
        if fresh {
            let run = self.new_run(id, ctx.clone()).await;
            self.runs.insert(id, run);
        }
        let Some(run) = self.runs.get_mut(&id) else {
            return;
        };
        let was_on = run.ctx.settings.auto_ref;
        let on = ctx.settings.auto_ref;
        // The room hears the match switch between the bot and a referee (unless the bot
        // switched it itself and said why). Switched back, the auto referee resumes the
        // next time it's driven (`drive`).
        let switched = !fresh && on != was_on && !ctx.concluded();
        let told_off = std::mem::take(&mut run.auto_told_off);
        if switched && on {
            run.auto_live = false;
        }
        run.ctx = ctx;
        run.dirty = true;
        match synced_at {
            None => run.osu = None,
            Some(at)
                if run.osu.as_ref().is_some_and(|(o, _)| *o == at)
                    || run.osu_loading == Some(at) => {}
            // Synced before this run started (a restart): osu!'s record is read again.
            Some(at) => {
                run.osu_loading = Some(at);
                let rooms: Vec<i64> = self.match_rooms(id).iter().map(|r| r.mp_id).collect();
                let app = self.app.clone();
                let inbox = self.inbox.clone();
                tokio::spawn(async move {
                    match read_osu_games(&app, &rooms).await {
                        Ok(games) => {
                            let _ = inbox.send(Request::OsuGames {
                                match_id: id,
                                at,
                                games,
                                force: false,
                            });
                        }
                        Err(err) => {
                            tracing::warn!(error = ?err, match_id = %id, "could not read osu!'s record of a match")
                        }
                    }
                });
            }
        }
        self.count(id);
        if switched && (on || !told_off) {
            self.auto_send(
                id,
                if on {
                    "Otomatik hakem tekrar devrede: maç otomatik modda, bot kaldığı yerden devam ediyor."
                } else {
                    "Maç manuel moda alındı: bundan sonra maçı hakem yönetiyor, otomatik hakem durdu."
                }
                .into(),
            );
        }
        if on && self.auto_on(id).is_some() {
            // Resumes at once rather than on the next tick.
            self.drive(id, |_, _, _| {}).await;
        }
    }

    // A match the bots start running, with where its auto referee stood.
    pub(super) async fn new_run(&self, id: Uuid, ctx: MatchContext) -> Run {
        let saved = db::autoref::find_state(&self.app.db, id)
            .await
            .unwrap_or_else(|err| {
                tracing::error!(error = ?err, match_id = %id, "could not read the auto referee's state");
                None
            });
        let auto = saved
            .clone()
            .and_then(|v| {
                serde_json::from_value(v)
                    .map_err(|err| {
                        tracing::warn!(error = %err, match_id = %id, "the auto referee's saved state is unreadable, starting over")
                    })
                    .ok()
            })
            .unwrap_or_default();
        Run {
            ctx,
            osu: None,
            osu_loading: None,
            last_key: None,
            force_save: false,
            dirty: true,
            auto,
            auto_saved: saved,
            auto_live: false,
            auto_told_off: false,
        }
    }

    // The match's rooms, oldest first.
    pub(super) fn match_rooms(&self, id: Uuid) -> Vec<&Room> {
        let mut rooms: Vec<&Room> = self.rooms.values().filter(|r| r.match_id == id).collect();
        rooms.sort_by_key(|r| (r.added_at, r.mp_id));
        rooms
    }

    pub(super) fn latest_room(&self, id: Uuid) -> Option<i64> {
        self.match_rooms(id).last().map(|r| r.mp_id)
    }

    pub(super) fn access(&self, room: &Room) -> RoomAccess {
        if room.closed {
            RoomAccess::Closed
        } else if room.bot.is_none() {
            RoomAccess::NoBot
        } else if !self.bot_online(room.bot) {
            RoomAccess::Offline
        } else if room.joined {
            RoomAccess::In
        } else if room.refused {
            RoomAccess::Refused
        } else {
            RoomAccess::Joining
        }
    }

    pub(super) fn bot_name(&self, bot: Option<Uuid>) -> Option<String> {
        bot.and_then(|b| self.bots.get(&b))
            .map(|b| b.login.username.clone())
    }

    // The match's room with where its auto referee stands.
    pub(super) fn view_of(&self, id: Uuid) -> Option<LiveRoom> {
        let mut view = self.base_view(id)?;
        let run = self.runs.get(&id)?;
        let enabled = run.ctx.settings.auto_ref;
        view.autoref = self.with_snapshot(id, |s| run.auto.view(s, enabled));
        Some(view)
    }

    pub(super) fn base_view(&self, id: Uuid) -> Option<LiveRoom> {
        let run = self.runs.get(&id)?;
        let rooms: Vec<ViewRoom> = self
            .match_rooms(id)
            .into_iter()
            .map(|r| ViewRoom {
                mp_id: r.mp_id,
                state: &r.state,
                bot: self.bot_name(r.bot),
                access: self.access(r),
            })
            .collect();
        Some(view::build(
            &run.ctx,
            &rooms,
            run.osu.as_ref().map(|(_, g)| g),
        ))
    }

    // Counts the match's score again: saves it when it changed (not on the first reading,
    // which would overwrite a score entered by hand), and puts the tiebreaker on when due.
    pub(super) fn count(&mut self, id: Uuid) {
        run_dirty(&mut self.runs, id);
        let Some(run) = self.runs.get(&id) else {
            return;
        };
        let ctx = &run.ctx;
        if ctx.kind == MatchKind::Qualifier {
            return self.count_lobby(id);
        }
        let Some([a, b]) = ctx
            .sides
            .clone()
            .filter(|_| ctx.is_bracket() && !ctx.forfeit)
        else {
            return;
        };
        let rooms: Vec<ViewRoom> = self
            .match_rooms(id)
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
            entries: &ctx.entries,
            picks: &ctx.picks,
            roster: &ctx.roster,
            ignored: &ctx.ignored,
            manual: &ctx.manual,
            team1_id: a.id,
            team2_id: b.id,
        });
        let key = score_key(Some(t.team1), Some(t.team2), &t.maps);
        let first = run.last_key.is_none();
        let changed = run.last_key.as_deref() != Some(key.as_str());
        let save = key != ctx.saved_key && (run.force_save || (!first && changed));
        let tiebreaker = ctx
            .best_of
            .filter(|&bo| standing_of(bo, t.team1, t.team2) == Standing::Tiebreaker)
            .and_then(|_| ctx.entries.iter().find(|e| e.tiebreaker).cloned())
            .map(|tb| {
                let names = [a.name.as_str(), b.name.as_str()];
                (
                    format!("{}-{}", t.team1, t.team2),
                    tb.beatmap_id,
                    vec![
                        format!(
                            "{} | Tiebreaker!",
                            score_message(names, [t.team1, t.team2], None)
                        ),
                        map_command(tb.beatmap_id, &tb.mode),
                        mods_command(&tb.mods, tb.freemod, ctx.settings.no_fail),
                    ],
                )
            });
        let Some(run) = self.runs.get_mut(&id) else {
            return;
        };
        run.last_key = Some(key);
        run.force_save = false;
        if save {
            let _ = self.saver.send(SaveJob {
                match_id: id,
                save: Save::Score {
                    team1: t.team1,
                    team2: t.team2,
                    maps: t.maps,
                },
                reply: None,
            });
        }
        // The auto referee opens the tiebreaker itself, when it's due.
        if let Some(cue) = tiebreaker.filter(|_| self.auto_on(id).is_none()) {
            self.cue_tiebreaker(id, cue);
        }
    }

    // A lobby's finished pool maps are saved whenever they differ from the site's, the
    // first reading included: nobody enters them by hand.
    fn count_lobby(&mut self, id: Uuid) {
        let Some(run) = self.runs.get(&id) else {
            return;
        };
        let rooms: Vec<ViewRoom> = self
            .match_rooms(id)
            .into_iter()
            .map(|r| ViewRoom {
                mp_id: r.mp_id,
                state: &r.state,
                bot: None,
                access: RoomAccess::In,
            })
            .collect();
        let finished = finished_of(&rooms, run.osu.as_ref().map(|(_, g)| g));
        let rows = lobby_rows(
            &finished,
            &run.ctx.entries,
            &run.ctx.roster,
            &run.ctx.ignored,
        );
        let key = lobby_key(&rows);
        let save = key != run.ctx.saved_key && run.last_key.as_deref() != Some(key.as_str());
        let Some(run) = self.runs.get_mut(&id) else {
            return;
        };
        run.last_key = Some(key);
        if save {
            let _ = self.saver.send(SaveJob {
                match_id: id,
                save: Save::Lobby(rows),
                reply: None,
            });
        }
    }

    // The tiebreaker goes on between maps, once per score, and only while the room still
    // has the map that ended (or none): a map the referee put on since is theirs.
    fn cue_tiebreaker(&mut self, id: Uuid, (key, beatmap_id, lines): (String, i64, Vec<String>)) {
        let Some(mp_id) = self.latest_room(id) else {
            return;
        };
        let ready = self.rooms.get(&mp_id).is_some_and(|r| {
            !r.closed && r.joined && !r.state.playing && r.tiebreaker_done.as_ref() != Some(&key)
        });
        if !ready || !self.bot_online(self.rooms[&mp_id].bot) {
            return;
        }
        let Some(room) = self.rooms.get_mut(&mp_id) else {
            return;
        };
        room.tiebreaker_done = Some(key);
        let current = room.state.beatmap_id;
        let last = room.state.results.last().and_then(|r| r.beatmap_id);
        if current == Some(beatmap_id) || (current.is_some() && current != last) {
            return;
        }
        let Some(bot) = room.bot else {
            return;
        };
        for line in lines {
            self.bot_send(
                bot,
                room_channel(mp_id),
                line,
                Some(mp_id),
                Priority::Normal,
            );
        }
    }
}
