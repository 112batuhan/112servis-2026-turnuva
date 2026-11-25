//! Driving the auto referee (crate::bancho::autoref): handing it a snapshot of the match,
//! carrying out the actions it returns, and the private messages players send the bots.

use std::collections::HashSet;

use time::OffsetDateTime;
use tokio::time::Instant;
use uuid::Uuid;

use crate::bancho::{
    autoref::{Action, AutoRef, Snapshot, WarmupMap, Web},
    connection::{
        bot::Priority,
        irc::{room_channel, valid_name},
    },
    names::same_name,
    room::commands::player_ref,
    tally::{osu_results, tally, TallyInput},
    view::{finished_of, RoomAccess, ViewRoom},
    PlayerAction, Request,
};
use crate::{
    db::{self, autoref::LogLevel, matches::MatchKind},
    discord::jobs::Alert,
    error::{AppError, GeneralError, MatchError, PlayerError},
    osu_api,
    services::{self, bracket::Mark},
};

use super::read_osu_games;
use super::{AutoWrite, Manager, INVITE_EVERY};

// The match's room page, where its players choose their warmups, bans and picks
// (frontend/src/pages/public/match/room/).
pub fn room_page_url(frontend_url: &str, id: Uuid) -> String {
    format!("{}/match/{id}/oda", frontend_url.trim_end_matches('/'))
}

// A player's answer: what the room page hears back.
type Reply = tokio::sync::oneshot::Sender<Result<(), AppError>>;

impl Manager {
    pub(super) fn auto_log(&self, id: Uuid, level: LogLevel, message: String) {
        let _ = self.autoref.send(AutoWrite::Log(id, level, message));
    }

    // The latest room, when the auto referee runs the match now: it's in auto mode, not
    // over, and the room's bot is in the room.
    pub(super) fn auto_on(&self, id: Uuid) -> Option<i64> {
        let run = self.runs.get(&id)?;
        let ctx = &run.ctx;
        if !ctx.settings.auto_ref || ctx.concluded() || (ctx.is_bracket() && ctx.sides.is_none()) {
            return None;
        }
        let mp_id = self.latest_room(id)?;
        let room = self.rooms.get(&mp_id)?;
        (room.joined && !room.closed && self.bot_online(room.bot)).then_some(mp_id)
    }

    // The match as its auto referee reads it, for `f`. None without a room.
    pub(super) fn with_snapshot<R>(&self, id: Uuid, f: impl FnOnce(&Snapshot) -> R) -> Option<R> {
        let run = self.runs.get(&id)?;
        let view = self.base_view(id)?;
        let room = *self.match_rooms(id).last()?;
        let ctx = &run.ctx;
        let played: HashSet<Uuid> = view
            .finished
            .iter()
            .filter(|f| f.counted)
            .filter_map(|f| f.map_id)
            .collect();
        let finished: HashSet<Uuid> = view.finished.iter().filter_map(|f| f.map_id).collect();
        let page = room_page_url(&self.app.config.frontend_url, id);
        let snapshot = Snapshot {
            now: OffsetDateTime::now_utc(),
            bracket: ctx.is_bracket(),
            starts_at: ctx.starts_at,
            room_added_at: room.added_at,
            teams: &ctx.teams,
            roster: &ctx.roster,
            entries: &ctx.entries,
            bans: &ctx.bans,
            picks: &ctx.picks,
            missed: &ctx.missed,
            turn: view.next_turn,
            score: view.tally.as_ref().map(|t| (t.team1, t.team2)),
            best_of: ctx.best_of,
            played: &played,
            finished: &finished,
            room: &room.state,
            no_fail: ctx.settings.no_fail,
            first_step_team: ctx.draft_order.steps.first().map_or(1, |s| s.team),
            page: &page,
            stand_ins: self.app.config.dev_login,
        };
        Some(f(&snapshot))
    }

    // Runs `f` on the match's auto referee with the match as it stands, saves where it
    // stands, and does what it asked. An auto referee that wasn't running the match the
    // last time (manual mode, its bot away, a restart) and is now resumes first.
    pub(super) async fn drive(
        &mut self,
        id: Uuid,
        f: impl FnOnce(&mut AutoRef, &Snapshot, &mut Vec<Action>),
    ) {
        let on = self.auto_on(id).is_some();
        let Some(run) = self.runs.get_mut(&id) else {
            return;
        };
        let resume = on && !run.auto_live;
        run.auto_live = on;
        let mut auto = std::mem::take(&mut run.auto);
        let mut out = Vec::new();
        self.with_snapshot(id, |s| {
            if resume {
                auto.resume(s, &mut out);
            }
            f(&mut auto, s, &mut out)
        });
        let Some(run) = self.runs.get_mut(&id) else {
            return;
        };
        run.auto = auto;
        if let Ok(value) = serde_json::to_value(&run.auto) {
            if run.auto_saved.as_ref() != Some(&value) {
                run.auto_saved = Some(value.clone());
                run.dirty = true;
                let _ = self.autoref.send(AutoWrite::State(id, value));
            }
        }
        self.act(id, out).await;
    }

    // Does what the auto referee asked, in order. A change to the match is read back on
    // the next tick.
    async fn act(&mut self, id: Uuid, actions: Vec<Action>) {
        for action in actions {
            let result: Result<(), AppError> = match action {
                Action::Send(text) => {
                    self.auto_send(id, text);
                    Ok(())
                }
                Action::Log(level, message) => {
                    self.auto_log(id, level, message);
                    Ok(())
                }
                Action::Ban { map_id, team_id } => {
                    self.pending.insert(id);
                    services::bracket::mark(&self.app, id, map_id, team_id, Mark::Ban).await
                }
                Action::Pick { map_id, team_id } => {
                    self.pending.insert(id);
                    services::bracket::mark(&self.app, id, map_id, team_id, Mark::Pick).await
                }
                Action::FirstTurn(team_id) => {
                    self.pending.insert(id);
                    services::referee::set_next_pick(&self.app, id, Some(team_id)).await
                }
                Action::Missed { team_id, kind } => {
                    self.pending.insert(id);
                    services::bracket::miss(&self.app, id, team_id, kind).await
                }
                Action::FetchOsu => {
                    self.fetch_osu(id);
                    Ok(())
                }
                Action::WarmupMap(beatmap_id) => {
                    self.fetch_warmup(id, beatmap_id);
                    Ok(())
                }
                Action::Forfeit(slot) => {
                    services::autoref::forfeit_no_show(&self.app, id, slot).await
                }
                Action::RemoveTeams(teams) => {
                    self.pending.insert(id);
                    services::autoref::lobby_no_show(&self.app, id, &teams).await
                }
                Action::Conclude => self.auto_conclude(id).await,
                Action::Report {
                    user_id,
                    team_id,
                    text,
                } => {
                    services::autoref::alert(
                        &self.app,
                        id,
                        Alert::Report,
                        Some((user_id, team_id)),
                        Some(&text),
                    )
                    .await
                }
                Action::Stop {
                    user_id,
                    team_id,
                    username,
                } => {
                    self.pending.insert(id);
                    let auto = self.runs.get(&id).is_some_and(|r| r.ctx.settings.auto_ref);
                    // The room heard why already.
                    if let Some(run) = self.runs.get_mut(&id) {
                        run.auto_told_off = auto;
                    }
                    services::autoref::emergency_stop(
                        &self.app,
                        id,
                        (user_id, team_id),
                        &username,
                        auto,
                    )
                    .await
                }
                Action::Handover(reason) => {
                    self.pending.insert(id);
                    if let Some(run) = self.runs.get_mut(&id) {
                        run.auto_told_off = true;
                    }
                    self.auto_send(
                        id,
                        "Otomatik hakem durdu, maçı bir hakem devralacak.".into(),
                    );
                    services::autoref::handover(&self.app, id, &reason).await
                }
            };
            if let Err(err) = result {
                tracing::error!(error = ?err, match_id = %id, "the auto referee's action failed");
                self.auto_log(id, LogLevel::Error, format!("İşlem başarısız: {err}"));
            }
        }
    }

    // What a player did on the match's room page. A warmup's beatmap is checked against
    // the turn first, then described by osu! (off the manager's loop), then chosen
    // (`player_warmup`).
    pub(super) async fn player_action(
        &mut self,
        id: Uuid,
        user_id: Uuid,
        action: PlayerAction,
        reply: Reply,
    ) {
        if !self.runs.contains_key(&id) || self.latest_room(id).is_none() {
            let _ = reply.send(Err(PlayerError::NotRunning.into()));
            return;
        }
        let web = match action {
            PlayerAction::Draft { map_id } => Web::Draft(map_id),
            PlayerAction::SkipWarmup => Web::SkipWarmup,
            PlayerAction::Abort => Web::Abort,
            PlayerAction::Stop => Web::Stop,
            PlayerAction::Report { text } => Web::Report(text),
            PlayerAction::Warmup { beatmap_id } => {
                if self.auto_on(id).is_none() {
                    let _ = reply.send(Err(PlayerError::Manual.into()));
                    return;
                }
                let Some(run) = self.runs.get(&id) else {
                    return;
                };
                let auto = &run.auto;
                let checked = self
                    .with_snapshot(id, |s| auto.check_warmup(s, user_id))
                    .unwrap_or(Err(PlayerError::NotRunning));
                if let Err(err) = checked {
                    let _ = reply.send(Err(err.into()));
                    return;
                }
                match auto.warmup_known(beatmap_id) {
                    Some(map) => Web::Warmup(beatmap_id, map),
                    None => return self.describe_warmup(id, user_id, beatmap_id, reply),
                }
            }
        };
        let result = self.web(id, user_id, web).await;
        let _ = reply.send(result);
    }

    // The warmup osu! described, chosen now (its turn is checked again).
    pub(super) async fn player_warmup(
        &mut self,
        id: Uuid,
        user_id: Uuid,
        beatmap_id: i64,
        map: WarmupMap,
        reply: Reply,
    ) {
        let result = self.web(id, user_id, Web::Warmup(beatmap_id, map)).await;
        let _ = reply.send(result);
    }

    async fn web(&mut self, id: Uuid, user_id: Uuid, web: Web) -> Result<(), AppError> {
        let enabled = self.auto_on(id).is_some();
        let mut result = Err(PlayerError::NotRunning);
        self.drive(id, |auto, s, out| {
            result = auto.web(s, user_id, web, enabled, out)
        })
        .await;
        result.map_err(Into::into)
    }

    // Asks osu! for a warmup beatmap's length, set and mode, then hands it back to the
    // manager to be chosen; a beatmap osu! doesn't have, or not osu!standard, is refused
    // here.
    fn describe_warmup(&self, id: Uuid, user_id: Uuid, beatmap_id: i64, reply: Reply) {
        let app = self.app.clone();
        let inbox = self.inbox.clone();
        tokio::spawn(async move {
            let described = async {
                // The page hears osu! couldn't be reached, not why the app's token failed.
                let token =
                    osu_api::token::app_token(&app.osu_client, &app.oauth_http, &app.osu_app_token)
                        .await
                        .map_err(|err| {
                            tracing::warn!(error = ?err, "no osu! token for a player's warmup");
                            AppError::from(GeneralError::OsuApiUnreachable)
                        })?;
                osu_api::beatmap::fetch(&app.http_client, &token, beatmap_id).await
            }
            .await;
            match described {
                Ok(map) if map.mode != "osu" => {
                    let _ = reply.send(Err(PlayerError::WarmupNotStandard.into()));
                }
                Ok(map) => {
                    let _ = inbox.send(Request::PlayerWarmup {
                        match_id: id,
                        user_id,
                        beatmap_id,
                        map: WarmupMap {
                            seconds: map.total_length,
                            beatmapset_id: map.beatmapset_id,
                        },
                        reply,
                    });
                }
                Err(err) => {
                    let _ = reply.send(Err(err));
                }
            }
        });
    }

    // A line for the match's latest room, through its bot.
    pub(super) fn auto_send(&self, id: Uuid, text: String) {
        let Some(mp_id) = self.latest_room(id) else {
            return;
        };
        if let Some(bot) = self.rooms.get(&mp_id).and_then(|r| r.bot) {
            self.bot_send(
                bot,
                room_channel(mp_id),
                text,
                Some(mp_id),
                Priority::Normal,
            );
        }
    }

    // osu!'s record of the rooms, read for the map that just finished; it takes the place
    // of the chat's maps like a referee's sync does.
    fn fetch_osu(&self, id: Uuid) {
        let rooms: Vec<i64> = self.match_rooms(id).iter().map(|r| r.mp_id).collect();
        let Some(&latest) = rooms.last() else {
            return;
        };
        let expected = self.rooms.get(&latest).map_or(0, |r| r.state.results.len());
        let app = self.app.clone();
        let inbox = self.inbox.clone();
        tokio::spawn(async move {
            let confirmed = match read_osu_games(&app, &rooms).await {
                // osu! has no record of the room (yet): the chat's maps stand.
                Ok(games) if !games.contains_key(&latest) => false,
                Ok(games) => {
                    let has_it = games
                        .get(&latest)
                        .is_some_and(|g| osu_results(g).len() >= expected);
                    // Sent to the manager ahead of AutorefOsu, over the same channel.
                    if let Err(err) = services::referee::apply_osu_sync(&app, id, games).await {
                        tracing::error!(error = ?err, match_id = %id, "could not mark the osu! sync")
                    }
                    has_it
                }
                Err(err) => {
                    tracing::warn!(error = ?err, match_id = %id, "could not read osu!'s record for the auto referee");
                    false
                }
            };
            let _ = inbox.send(Request::AutorefOsu {
                match_id: id,
                confirmed,
            });
        });
    }

    // A warmup beatmap's length and set, from osu!.
    fn fetch_warmup(&self, id: Uuid, beatmap_id: i64) {
        let app = self.app.clone();
        let inbox = self.inbox.clone();
        tokio::spawn(async move {
            let token = match osu_api::token::app_token(
                &app.osu_client,
                &app.oauth_http,
                &app.osu_app_token,
            )
            .await
            {
                Ok(token) => token,
                Err(err) => {
                    tracing::warn!(error = ?err, "no osu! token for a warmup's length");
                    return;
                }
            };
            match osu_api::beatmap::fetch(&app.http_client, &token, beatmap_id).await {
                Ok(map) => {
                    let _ = inbox.send(Request::AutorefWarmup {
                        match_id: id,
                        beatmap_id,
                        seconds: map.total_length,
                        beatmapset_id: map.beatmapset_id,
                    });
                }
                Err(err) => {
                    tracing::warn!(error = ?err, beatmap_id, "could not read a warmup's length")
                }
            }
        });
    }

    // The match is over: a bracket match's score is saved as counted, then it's
    // concluded (the room closes CLOSE_AFTER_CONCLUDED later).
    async fn auto_conclude(&mut self, id: Uuid) -> Result<(), AppError> {
        let Some(run) = self.runs.get(&id) else {
            return Ok(());
        };
        if run.ctx.kind == MatchKind::Qualifier {
            return services::matches::conclude_lobby(&self.app, id).await;
        }
        let Some((team1, team2, maps)) = self.counted(id) else {
            return Err(MatchError::NeedsBothTeams.into());
        };
        services::bracket::save_score(&self.app, id, Some(team1), Some(team2), Some(&maps)).await?;
        services::bracket::conclude(&self.app, id, None).await?;
        self.app.live.bracket_changed();
        Ok(())
    }

    // A bracket match's score as the rooms count it, with the maps it's counted from.
    fn counted(&self, id: Uuid) -> Option<(i32, i32, Vec<db::bracket::MapResultRow>)> {
        let run = self.runs.get(&id)?;
        let [a, b] = run.ctx.sides.clone()?;
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
            entries: &run.ctx.entries,
            picks: &run.ctx.picks,
            roster: &run.ctx.roster,
            ignored: &run.ctx.ignored,
            manual: &run.ctx.manual,
            team1_id: a.id,
            team2_id: b.id,
        });
        Some((t.team1, t.team2, t.maps))
    }

    // A player wrote to one of the bots: "-davet" invites them again to their match's
    // room, if that bot runs it.
    pub(super) fn private_message(&mut self, bot_id: Uuid, from: &str, text: &str) {
        if !text.trim().eq_ignore_ascii_case("-davet") || !valid_name(from) {
            return;
        }
        let found = self.runs.iter().find_map(|(id, run)| {
            let mp_id = self.latest_room(*id)?;
            let room = self.rooms.get(&mp_id)?;
            if room.bot != Some(bot_id) || room.closed || !room.joined || run.ctx.concluded() {
                return None;
            }
            let p = run
                .ctx
                .roster
                .iter()
                .find(|p| same_name(&p.username, from))?;
            Some((*id, mp_id, p.osu_id, p.username.clone()))
        });
        let Some((match_id, mp_id, osu_id, username)) = found else {
            self.bot_send(
                bot_id,
                from.to_string(),
                "Şu an seni bekleyen, bu botun yönettiği bir maç odası yok.".into(),
                None,
                Priority::Normal,
            );
            return;
        };
        let now = Instant::now();
        if self
            .invited
            .get(&osu_id)
            .is_some_and(|&t| now.duration_since(t) < INVITE_EVERY)
        {
            return;
        }
        self.invited.insert(osu_id, now);
        self.bot_send(
            bot_id,
            room_channel(mp_id),
            format!("!mp invite {}", player_ref(osu_id)),
            Some(mp_id),
            Priority::High,
        );
        self.bot_send(
            bot_id,
            from.to_string(),
            format!("Davet gönderildi: #mp_{mp_id}"),
            None,
            Priority::Normal,
        );
        self.auto_log(
            match_id,
            LogLevel::Info,
            format!("{username} -davet ile tekrar davet edildi."),
        );
    }
}
