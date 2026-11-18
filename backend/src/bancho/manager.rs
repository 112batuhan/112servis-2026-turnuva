//! The one task that runs every room. It owns the bots' connections, every room's
//! state and every running match, and takes everything in order: what the bots hear,
//! what the pages ask, and the site's changes (over the live hub). So there is no race
//! for who runs a room or saves a score: it does, once.

use std::collections::{HashMap, HashSet};

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::{interval, Duration, Instant, MissedTickBehavior},
};
use uuid::Uuid;

use super::{
    autoref::{self, Action, AutoRef, Snapshot},
    bot::{self, BotCmd, BotConn, BotEvent, Priority},
    commands::{
        add_ref_command, kick_command, map_command, match_slots, mods_command, move_commands,
        place_commands, player_ref, remove_ref_command, Placement, HEAD_TO_HEAD, TEAM_VS,
    },
    context::{self, MatchContext},
    irc::{normalize_name, room_channel, room_id, same_name, valid_name, BANCHO_BOT},
    room::{
        parse_created, parse_joined, parse_left, read_lobby, RoomSlot, RoomState, MATCH_STARTED,
    },
    tally::{lobby_key, lobby_rows, race_of, score_key, score_message, tally, Race, TallyInput},
    view::{self, finished_of, LiveRoom, RoomAccess, ViewRoom},
    BotRoom, BotState, BotStatus, Request, RoomCommand, ROOM_LIMIT,
};
use crate::{
    db::{
        self, autoref::LogLevel, bancho::BotLogin, matches::MatchKind, stats::QualifierMapRow,
        user::StaffMember,
    },
    discord::jobs::Alert,
    error::{AppError, BanchoError, GeneralError, MatchError, RoomError},
    live::Change,
    osu_api,
    osu_api::matches::MatchGame,
    role::Role,
    room_scores,
    services::{self, bracket::Mark},
    supervise::{self, SharedReceiver},
    AppState,
};

// How often timers are checked and changed rooms pushed to the pages.
const TICK: Duration = Duration::from_millis(200);
// Every running match is read again this often, in case a change didn't say so.
const RESYNC_EVERY: Duration = Duration::from_secs(60);
// How long BanchoBot gets to answer !mp make.
const MAKE_TIMEOUT: Duration = Duration::from_secs(20);
// How long a bot that Bancho says is full is left out of new rooms.
const FULL_BACKOFF: Duration = Duration::from_secs(15 * 60);
// A concluded match's room is closed this long after, so the last lines (a result, a
// "gg") still reach it and a conclusion clicked by mistake can be undone; it then stops
// counting against its bot. The players keep talking in the match's Discord channel.
const CLOSE_AFTER_CONCLUDED: TimeDuration = TimeDuration::seconds(30);
// A room replaced by a newer one is closed this long after, while players move over.
const CLOSE_AFTER_REPLACED: TimeDuration = TimeDuration::minutes(15);
// The auto referee opens a match's room this long before its time (and not for a match
// whose time passed longer ago than OPEN_LATE), checking every OPEN_CHECK; a room it
// couldn't make is tried again after OPEN_RETRY.
const OPEN_BEFORE: i64 = 10 * 60;
const OPEN_LATE: i64 = 30 * 60;
const OPEN_CHECK: Duration = Duration::from_secs(15);
const OPEN_RETRY: Duration = Duration::from_secs(60);
// How often the auto referee's clock is checked.
const AUTOREF_TICK: Duration = Duration::from_secs(1);
// A player can ask for an invite (-davet) this often.
const INVITE_EVERY: Duration = Duration::from_secs(20);

struct Bot {
    login: BotLogin,
    conn: Option<(u64, BotConn)>,
    online: bool,
    online_since: Option<OffsetDateTime>,
    error: Option<String>,
    // The password Bancho refused; it's only tried again once it changes or the bot is
    // switched off and on.
    refused_password: Option<String>,
    full_until: Option<Instant>,
    // A !mp make waiting for BanchoBot's answer: one at a time, since the answer
    // doesn't say which request it's for.
    making: Option<Making>,
    // Rooms it joined that aren't known yet (made just now).
    joined_unknown: HashSet<i64>,
}

struct Making {
    match_id: Uuid,
    reply: oneshot::Sender<Result<i64, AppError>>,
    deadline: Instant,
    // Bots already tried for this room.
    tried: Vec<Uuid>,
    // The auto referee asked for it: Discord names the bot to the players.
    auto: bool,
}

struct Room {
    mp_id: i64,
    match_id: Uuid,
    bot: Option<Uuid>,
    bot_made: bool,
    added_at: OffsetDateTime,
    closed: bool,
    // The bot is in its channel.
    joined: bool,
    // Bancho wouldn't let the bot in.
    refused: bool,
    state: RoomState,
    // When the bot closes it (or, if it didn't make it, leaves it).
    close_at: Option<OffsetDateTime>,
    // Who joined under a name no member has, waiting for !mp settings to give their id.
    unknown: HashSet<String>,
    // Who the bot sent !mp addref for, by normalize_name.
    refs_asked: HashSet<String>,
    // The score the tiebreaker was last put on for.
    tiebreaker_done: Option<String>,
}

struct Run {
    ctx: MatchContext,
    // osu!'s record of the rooms as of the last sync, by room.
    osu: Option<(OffsetDateTime, HashMap<i64, Vec<MatchGame>>)>,
    osu_loading: Option<OffsetDateTime>,
    // The score last counted, None until the first reading.
    last_key: Option<String>,
    force_save: bool,
    dirty: bool,
    // The auto referee's state, and as it was last saved.
    auto: AutoRef,
    auto_saved: Option<serde_json::Value>,
}

struct Manager {
    app: AppState,
    inbox: mpsc::UnboundedSender<Request>,
    events: mpsc::UnboundedSender<BotEvent>,
    bots: HashMap<Uuid, Bot>,
    conns: HashMap<u64, Uuid>,
    next_conn: u64,
    rooms: HashMap<i64, Room>,
    runs: HashMap<Uuid, Run>,
    staff: Vec<StaffMember>,
    staff_loaded: bool,
    // Changes to read again on the next tick.
    pending: HashSet<Uuid>,
    pending_all: bool,
    status_changed: bool,
    chat: mpsc::UnboundedSender<ChatWrite>,
    saver: mpsc::UnboundedSender<SaveJob>,
    autoref: mpsc::UnboundedSender<AutoWrite>,
    // Matches the auto referee tried to open a room for, and when.
    open_tried: HashMap<Uuid, Instant>,
    last_open_check: Option<Instant>,
    last_autoref_tick: Option<Instant>,
    // When each player last asked for an invite, by osu! id.
    invited: HashMap<i64, Instant>,
}

// The auto referee's state and log lines, written in order.
enum AutoWrite {
    State(Uuid, serde_json::Value),
    Log(Uuid, LogLevel, String),
}

struct ChatWrite {
    mp_id: i64,
    from: String,
    text: String,
    action: bool,
    bot: Uuid,
    at: OffsetDateTime,
}

struct SaveJob {
    match_id: Uuid,
    save: Save,
    reply: Option<oneshot::Sender<Result<(), AppError>>>,
}

enum Save {
    // A bracket match's score and the maps it was counted from.
    Score {
        team1: i32,
        team2: i32,
        maps: Vec<db::bracket::MapResultRow>,
    },
    // A qualifier lobby's finished pool maps.
    Lobby(Vec<QualifierMapRow>),
}

// Supervised (crate::supervise): after a panic a new manager takes over the same inbox
// and rebuilds the rooms from their saved chat, as at startup.
pub(super) async fn run(app: AppState, inbox: SharedReceiver<Request>) {
    let mut inbox = inbox.lock().await;
    let (events_tx, mut events) = mpsc::unbounded_channel();
    let mut changes = app.live.changes();
    let mut manager = Manager {
        inbox: app.bancho.tx.clone(),
        events: events_tx,
        bots: HashMap::new(),
        conns: HashMap::new(),
        next_conn: 1,
        rooms: HashMap::new(),
        runs: HashMap::new(),
        staff: Vec::new(),
        staff_loaded: false,
        pending: HashSet::new(),
        pending_all: false,
        status_changed: false,
        chat: spawn_chat_writer(app.clone()),
        saver: spawn_saver(app.clone()),
        autoref: spawn_autoref_writer(app.clone()),
        open_tried: HashMap::new(),
        last_open_check: None,
        last_autoref_tick: None,
        invited: HashMap::new(),
        app,
    };
    manager.reload_bots().await;
    manager.resync().await;
    let app = manager.app.clone();
    tokio::spawn(async move {
        if let Err(err) = backfill_lobbies(&app).await {
            tracing::error!(error = ?err, "could not save the concluded lobbies' results");
        }
    });

    let mut tick = interval(TICK);
    tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let mut resync = interval(RESYNC_EVERY);
    resync.tick().await;
    loop {
        tokio::select! {
            Some(request) = inbox.recv() => manager.request(request).await,
            Some(event) = events.recv() => manager.event(event).await,
            change = changes.recv() => match change {
                Ok(Change::Match(id)) => { manager.pending.insert(id); }
                Ok(Change::All) | Err(broadcast::error::RecvError::Lagged(_)) => manager.pending_all = true,
                Err(broadcast::error::RecvError::Closed) => {}
            },
            _ = tick.tick() => manager.tick().await,
            _ = resync.tick() => manager.pending_all = true,
        }
    }
}

// Saves room lines in order, then sends each to the pages watching its match.
fn spawn_chat_writer(app: AppState) -> mpsc::UnboundedSender<ChatWrite> {
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
fn spawn_autoref_writer(app: AppState) -> mpsc::UnboundedSender<AutoWrite> {
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
fn spawn_saver(app: AppState) -> mpsc::UnboundedSender<SaveJob> {
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
async fn save_lobby(
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

// Saves the results of the concluded lobbies that have none, read from their saved chat:
// the bots only save a lobby's maps while they run it. Run at startup and after a dev
// reseed.
pub async fn backfill_lobbies(app: &AppState) -> Result<(), AppError> {
    for id in db::stats::lobbies_without_results(&app.db).await? {
        let Some(ctx) = context::load(&app.db, id).await? else {
            continue;
        };
        let Some(m) = db::matches::find_match(&app.db, id).await? else {
            continue;
        };
        let states = chat_states(app, &m.mp_ids).await?;
        let rooms: Vec<ViewRoom> = states
            .iter()
            .map(|(mp_id, state)| ViewRoom {
                mp_id: *mp_id,
                state,
                bot: None,
                access: RoomAccess::Closed,
            })
            .collect();
        let rows = lobby_rows(
            &finished_of(&rooms, None),
            &ctx.entries,
            &ctx.roster,
            &ctx.ignored,
        );
        if !rows.is_empty() {
            save_lobby(app, id, &rows).await?;
        }
    }
    Ok(())
}

// Each room as its saved chat describes it.
async fn chat_states(app: &AppState, mp_ids: &[i64]) -> Result<Vec<(i64, RoomState)>, AppError> {
    let mut states = Vec::new();
    for &mp_id in mp_ids {
        let lines = db::chat::list_messages(&app.db, mp_id).await?;
        let state = read_lobby(
            lines
                .iter()
                .filter(|l| is_bancho_bot(&l.sender) && !l.action)
                .map(|l| (l.text.as_str(), l.sent_at)),
        );
        states.push((mp_id, state));
    }
    Ok(states)
}

fn is_bancho_bot(name: &str) -> bool {
    name.eq_ignore_ascii_case(BANCHO_BOT)
}

impl Manager {
    // ---------- bots ----------

    async fn reload_bots(&mut self) {
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
    fn bot_offline(&mut self, id: Uuid) {
        for room in self.rooms.values_mut().filter(|r| r.bot == Some(id)) {
            room.joined = false;
        }
        self.mark_bot_rooms(id);
        self.status_changed = true;
    }

    fn mark_bot_rooms(&mut self, id: Uuid) {
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

    fn bot_send(
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

    fn bot_raw(&self, bot: Uuid, cmd: BotCmd) {
        if let Some((_, conn)) = self.bots.get(&bot).and_then(|b| b.conn.as_ref()) {
            let _ = conn.tx.send(cmd);
        }
    }

    fn bot_online(&self, bot: Option<Uuid>) -> bool {
        bot.and_then(|b| self.bots.get(&b))
            .is_some_and(|b| b.online)
    }

    fn is_bot_name(&self, name: &str) -> bool {
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
    fn pick_bot(&self, skip: &[Uuid], making: bool) -> Result<Uuid, AppError> {
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

    // ---------- rooms and runs ----------

    async fn resync(&mut self) {
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
                    unknown: HashSet::new(),
                    refs_asked: HashSet::new(),
                    tiebreaker_done: None,
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
                let close_at = match (t.concluded_at, rooms.get(i + 1)) {
                    (Some(at), _) => Some(at + CLOSE_AFTER_CONCLUDED),
                    (None, Some(next)) => Some(next.added_at + CLOSE_AFTER_REPLACED),
                    (None, None) => None,
                };
                if let Some(room) = self.rooms.get_mut(&t.mp_id) {
                    room.close_at = close_at.filter(|_| !room.closed && room.bot.is_some());
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
        for id in self.runs.keys().copied().collect::<Vec<_>>() {
            if !running_set.contains(&id) {
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
    async fn reload(&mut self, id: Uuid) {
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
        // A run read back after a restart, or a match switched back to auto, picks up
        // where its auto referee left off.
        let resume = on
            && if fresh {
                !matches!(
                    run.auto.phase,
                    autoref::Phase::Waiting | autoref::Phase::Done
                )
            } else {
                !was_on
            };
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
        if resume {
            self.drive(id, |auto, s, out| auto.resume(s, out)).await;
        }
    }

    // A match the bots start running, with where its auto referee stood.
    async fn new_run(&self, id: Uuid, ctx: MatchContext) -> Run {
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
        }
    }

    // The match's rooms, oldest first.
    fn match_rooms(&self, id: Uuid) -> Vec<&Room> {
        let mut rooms: Vec<&Room> = self.rooms.values().filter(|r| r.match_id == id).collect();
        rooms.sort_by_key(|r| (r.added_at, r.mp_id));
        rooms
    }

    fn latest_room(&self, id: Uuid) -> Option<i64> {
        self.match_rooms(id).last().map(|r| r.mp_id)
    }

    fn access(&self, room: &Room) -> RoomAccess {
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

    fn bot_name(&self, bot: Option<Uuid>) -> Option<String> {
        bot.and_then(|b| self.bots.get(&b))
            .map(|b| b.login.username.clone())
    }

    // The match's room with where its auto referee stands.
    fn view_of(&self, id: Uuid) -> Option<LiveRoom> {
        let mut view = self.base_view(id)?;
        let run = self.runs.get(&id)?;
        let enabled = run.ctx.settings.auto_ref;
        view.autoref = self.with_snapshot(id, |s| run.auto.view(s, enabled));
        Some(view)
    }

    fn base_view(&self, id: Uuid) -> Option<LiveRoom> {
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
    fn count(&mut self, id: Uuid) {
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
            .filter(|&bo| race_of(bo, t.team1, t.team2) == Race::Tiebreaker)
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

    // ---------- what the bots hear ----------

    async fn event(&mut self, event: BotEvent) {
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

    fn room_closed(&mut self, mp_id: i64) {
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
        if !is_bancho_bot(&from) {
            if self.runs.contains_key(&match_id)
                && !self.is_bot_name(&from)
                && self.latest_room(match_id) == Some(mp_id)
            {
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

    // ---------- making rooms ----------

    fn start_making(&mut self, bot_id: Uuid, mut making: Making) {
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

    // BanchoBot made the room: it's the match's now, set up and on the site.
    async fn room_made(
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
                unknown: HashSet::new(),
                refs_asked: HashSet::new(),
                tiebreaker_done: None,
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
    fn set_up(&self, match_id: Uuid, mp_id: i64) {
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

    // ---------- automation ----------

    fn placements(ctx: &MatchContext) -> Vec<Placement> {
        ctx.roster
            .iter()
            .filter(|r| r.color.is_some())
            .map(|r| Placement {
                osu_id: r.osu_id,
                username: r.username.clone(),
                slot: r.home,
                team: r.color,
            })
            .collect()
    }

    // Staff who may be in a room without playing.
    fn staff_of(&self, slot: &RoomSlot) -> Option<&StaffMember> {
        self.staff.iter().find(|s| {
            [Role::Host, Role::Referee, Role::Streamer]
                .iter()
                .any(|r| s.roles.contains(r))
                && match slot.user_id {
                    Some(id) => s.osu_id == id,
                    None => same_name(&s.username, &slot.username),
                }
        })
    }

    fn is_member(&self, ctx: &MatchContext, slot: &RoomSlot) -> bool {
        ctx.roster.iter().any(|p| match slot.user_id {
            Some(id) => p.osu_id == id,
            None => same_name(&p.username, &slot.username),
        }) || self.staff_of(slot).is_some()
    }

    // What the room does by itself as BanchoBot reports it: reads everyone's mods as each
    // map starts, puts bracket players in their slot and colour as they join, makes every
    // holder of Referee who joins one of its referees (never a bot), and kicks whoever
    // joins without belonging there. Only in the match's latest room.
    fn automate(&mut self, match_id: Uuid, mp_id: i64, text: &str) {
        if self.latest_room(match_id) != Some(mp_id) {
            return;
        }
        let Some(bot) = self
            .rooms
            .get(&mp_id)
            .filter(|r| r.joined && !r.closed)
            .and_then(|r| r.bot)
        else {
            return;
        };
        if !self.bot_online(Some(bot)) {
            return;
        }
        let channel = room_channel(mp_id);
        if text == MATCH_STARTED {
            self.bot_send(
                bot,
                channel,
                "!mp settings".into(),
                Some(mp_id),
                Priority::Low,
            );
            return;
        }
        let ctx = self.runs[&match_id].ctx.clone();
        let mut out: Vec<String> = Vec::new();
        let room = &self.rooms[&mp_id];

        // !mp settings gave the ids of those who joined under an unknown name.
        let mut unknown = room.unknown.clone();
        let mut settle: Vec<RoomSlot> = Vec::new();
        if !unknown.is_empty() {
            for slot in &room.state.slots {
                let name = normalize_name(&slot.username);
                if slot.user_id.is_some() && unknown.remove(&name) {
                    settle.push(slot.clone());
                }
            }
            if let Some(left) = parse_left(text) {
                unknown.remove(&normalize_name(left));
            }
        }
        let mut refs_asked = room.refs_asked.clone();
        for slot in &settle {
            if let Some(p) = ctx.roster.iter().find(|p| Some(p.osu_id) == slot.user_id) {
                out.extend(self.place(&ctx, &room.state, slot, p.osu_id));
            } else if let Some(line) = self.add_ref(&room.state, slot, &mut refs_asked) {
                out.extend(line);
            } else if ctx.settings.auto_kick && self.staff_loaded && !self.is_member(&ctx, slot) {
                out.push(kick_command(slot));
            }
        }

        if let Some((name, _, _)) = parse_joined(text) {
            if let Some(slot) = room
                .state
                .slots
                .iter()
                .find(|s| same_name(&s.username, name))
                .cloned()
            {
                if let Some(p) = ctx.roster.iter().find(|p| same_name(&p.username, name)) {
                    out.extend(self.place(&ctx, &room.state, &slot, p.osu_id));
                } else if let Some(line) = self.add_ref(&room.state, &slot, &mut refs_asked) {
                    out.extend(line);
                } else if !self.is_member(&ctx, &slot)
                    && (ctx.settings.auto_kick || ctx.settings.auto_place)
                    && !self.is_bot_name(name)
                {
                    // A name no member has: a stranger, or someone who renamed on osu! since
                    // the site last saw them. Settled by the osu! id !mp settings gives.
                    unknown.insert(normalize_name(name));
                    out.push("!mp settings".into());
                }
            }
        }
        let Some(room) = self.rooms.get_mut(&mp_id) else {
            return;
        };
        room.unknown = unknown;
        room.refs_asked = refs_asked;
        for line in out {
            self.bot_send(bot, channel.clone(), line, Some(mp_id), Priority::Normal);
        }
    }

    // A bracket player to their slot and colour; Bancho barely answers !mp team, so the
    // room is read again after.
    fn place(
        &self,
        ctx: &MatchContext,
        state: &RoomState,
        slot: &RoomSlot,
        osu_id: i64,
    ) -> Vec<String> {
        if !ctx.is_bracket() || !ctx.settings.auto_place {
            return Vec::new();
        }
        let placements = Self::placements(ctx);
        let Some(p) = placements.iter().find(|p| p.osu_id == osu_id) else {
            return Vec::new();
        };
        let mover = RoomSlot {
            user_id: Some(osu_id),
            ..slot.clone()
        };
        let mut lines = move_commands(
            state,
            &mover,
            p.slot,
            p.team,
            Some(&match_slots(&placements)),
        );
        if !lines.is_empty() {
            lines.push("!mp settings".into());
        }
        lines
    }

    // A holder of Referee who joined the room is made one of its referees, once; by the
    // name the join gave. Some(lines) when they're a referee.
    fn add_ref(
        &self,
        state: &RoomState,
        slot: &RoomSlot,
        asked: &mut HashSet<String>,
    ) -> Option<Vec<String>> {
        let member = self
            .staff_of(slot)
            .filter(|s| s.roles.contains(&Role::Referee))?;
        if self.is_bot_name(&member.username) {
            return None;
        }
        let name = normalize_name(&slot.username);
        if state.is_ref(&slot.username) || !asked.insert(name) {
            return Some(Vec::new());
        }
        Some(vec![add_ref_command(&slot.username)])
    }

    // ---------- the auto referee ----------

    fn auto_log(&self, id: Uuid, level: LogLevel, message: String) {
        let _ = self.autoref.send(AutoWrite::Log(id, level, message));
    }

    // The latest room, when the auto referee runs the match now: it's in auto mode, not
    // over, and the room's bot is in the room.
    fn auto_on(&self, id: Uuid) -> Option<i64> {
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
    fn with_snapshot<R>(&self, id: Uuid, f: impl FnOnce(&Snapshot) -> R) -> Option<R> {
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
            turn: view.next_turn,
            score: view.tally.as_ref().map(|t| (t.team1, t.team2)),
            best_of: ctx.best_of,
            played: &played,
            room: &room.state,
            no_fail: ctx.settings.no_fail,
            first_step_team: ctx.draft_order.steps.first().map_or(1, |s| s.team),
        };
        Some(f(&snapshot))
    }

    // Runs `f` on the match's auto referee with the match as it stands, saves where it
    // stands, and does what it asked.
    async fn drive(&mut self, id: Uuid, f: impl FnOnce(&mut AutoRef, &Snapshot, &mut Vec<Action>)) {
        let Some(mut auto) = self.runs.get_mut(&id).map(|r| std::mem::take(&mut r.auto)) else {
            return;
        };
        let mut out = Vec::new();
        self.with_snapshot(id, |s| f(&mut auto, s, &mut out));
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
                    let saved = db::referee::set_next_pick(&self.app.db, id, Some(team_id)).await;
                    self.app.live.referee_changed(id);
                    saved.map_err(AppError::from)
                }
                Action::FetchOsu => {
                    self.fetch_osu(id);
                    Ok(())
                }
                Action::WarmupLength(beatmap_id) => {
                    self.fetch_length(id, beatmap_id);
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

    // A line for the match's latest room, through its bot.
    fn auto_send(&self, id: Uuid, text: String) {
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
                        .is_some_and(|g| crate::bancho::tally::osu_results(g).len() >= expected);
                    match db::referee::mark_osu_synced(&app.db, id).await {
                        Ok(at) => {
                            let _ = inbox.send(Request::OsuGames {
                                match_id: id,
                                at,
                                games,
                                force: true,
                            });
                            app.live.referee_changed(id);
                        }
                        Err(err) => {
                            tracing::error!(error = ?err, match_id = %id, "could not mark the osu! sync")
                        }
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

    // A warmup beatmap's length, from osu!.
    fn fetch_length(&self, id: Uuid, beatmap_id: i64) {
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
                    let _ = inbox.send(Request::AutorefLength {
                        match_id: id,
                        beatmap_id,
                        seconds: map.total_length,
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
    fn private_message(&mut self, bot_id: Uuid, from: &str, text: &str) {
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
            let making = self
                .bots
                .values()
                .any(|b| b.making.as_ref().is_some_and(|m| m.match_id == match_id));
            if making || self.open_tried.contains_key(&match_id) {
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

    // ---------- timers ----------

    async fn tick(&mut self) {
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

        // Rooms whose match is over (or that were replaced) are closed, or left.
        let wall = OffsetDateTime::now_utc();
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
            let Some(bot) = room.bot else {
                continue;
            };
            let made = room.bot_made;
            if made {
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

    // ---------- what the pages ask ----------

    async fn request(&mut self, request: Request) {
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
            Request::AutorefLength {
                match_id,
                beatmap_id,
                seconds,
            } => {
                self.drive(match_id, |auto, s, out| {
                    auto.warmup_length(s, beatmap_id, seconds, out)
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

    async fn create_room(
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
        match self.pick_bot(&[], true) {
            Ok(bot) => self.start_making(
                bot,
                Making {
                    match_id,
                    reply,
                    deadline: Instant::now(),
                    tried: Vec::new(),
                    auto,
                },
            ),
            Err(err) => {
                let _ = reply.send(Err(err));
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

fn run_dirty(runs: &mut HashMap<Uuid, Run>, id: Uuid) {
    if let Some(run) = runs.get_mut(&id) {
        run.dirty = true;
    }
}

// osu!'s record of each of the rooms; a room osu! has no record of is left out.
pub async fn read_osu_games(
    app: &AppState,
    rooms: &[i64],
) -> Result<HashMap<i64, Vec<MatchGame>>, AppError> {
    let token =
        osu_api::token::app_token(&app.osu_client, &app.oauth_http, &app.osu_app_token).await?;
    let mut games = HashMap::new();
    for &mp_id in rooms {
        match osu_api::matches::fetch_games(&app.http_client, &token, mp_id).await {
            Ok(g) => {
                games.insert(mp_id, g);
            }
            Err(AppError::OsuApiRequest(err)) if osu_api::matches::refused(&err) => {
                tracing::warn!(error = %err, mp_id, "osu! has no readable record of the room");
            }
            Err(err) => return Err(err),
        }
    }
    Ok(games)
}

// A match the bots don't run (it's over, or it has no room yet), read from its saved
// chat for the pages.
pub async fn static_view(app: &AppState, match_id: Uuid) -> Result<Option<LiveRoom>, AppError> {
    let Some(ctx) = context::load(&app.db, match_id).await? else {
        return Ok(None);
    };
    let Some(m) = db::matches::find_match(&app.db, match_id).await? else {
        return Ok(None);
    };
    if m.mp_ids.is_empty() {
        return Ok(None);
    }
    let states = chat_states(app, &m.mp_ids).await?;
    let rooms: Vec<ViewRoom> = states
        .iter()
        .map(|(mp_id, state)| ViewRoom {
            mp_id: *mp_id,
            state,
            bot: None,
            access: if ctx.kind == MatchKind::Qualifier || ctx.concluded() {
                RoomAccess::Closed
            } else {
                RoomAccess::NoBot
            },
        })
        .collect();
    Ok(Some(view::build(&ctx, &rooms, None)))
}
