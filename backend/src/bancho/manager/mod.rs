//! The one task that runs every room. It owns the bots' connections, every room's
//! state and every running match, and takes everything in order: what the bots hear,
//! what the pages ask, and the site's changes (over the live hub). So there is no race
//! for who runs a room or saves a score: it does, once.

mod auto_referee;
mod automation;
mod bots;
mod events;
mod making;
mod offline;
mod requests;
mod runs;
mod saving;
mod timers;

pub use offline::{backfill_lobbies, read_osu_games, static_view};

use std::collections::{HashMap, HashSet, VecDeque};

use time::{Duration as TimeDuration, OffsetDateTime};
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    time::{interval, Duration, Instant, MissedTickBehavior},
};
use uuid::Uuid;

use super::{
    autoref::AutoRef,
    connection::{
        bot::{BotConn, BotEvent},
        irc::BANCHO_BOT,
    },
    context::MatchContext,
    room::RoomState,
    Request,
};
use crate::{
    db::{self, autoref::LogLevel, bancho::BotLogin, stats::QualifierMapRow, user::StaffMember},
    error::AppError,
    live::Change,
    osu_api::matches::MatchGame,
    supervise::SharedReceiver,
    AppState,
};

use self::saving::{spawn_autoref_writer, spawn_chat_writer, spawn_saver};

// How often timers are checked and changed rooms pushed to the pages.
const TICK: Duration = Duration::from_millis(200);
// Every running match is read again this often, in case a change didn't say so.
const RESYNC_EVERY: Duration = Duration::from_secs(60);
// How long BanchoBot gets to answer !mp make.
const MAKE_TIMEOUT: Duration = Duration::from_secs(20);
// How long a room asked for while every bot was busy making one waits for a free bot.
const QUEUE_TIMEOUT: Duration = Duration::from_secs(60);
// How long a bot that Bancho says is full is left out of new rooms.
const FULL_BACKOFF: Duration = Duration::from_secs(15 * 60);
// A concluded match's room is closed this long after, so the last lines (a result, a
// "gg") still reach it and a conclusion clicked by mistake can be undone; it then stops
// counting against its bot. The players keep talking in the match's Discord channel.
// The room is told (autoref::CLOSING, and CLOSE_WARN_TEXT CLOSE_WARN before the end),
// and closed autoref::GRACE after the end it was told, like the auto referee's waits.
const CLOSE_AFTER_CONCLUDED: TimeDuration = TimeDuration::seconds(30);
const CLOSE_WARN: TimeDuration = TimeDuration::seconds(10);
const CLOSE_WARN_TEXT: &str = "Oda 10 saniye sonra kapanacak.";
const CLOSED_TEXT: &str = "Oda kapanıyor.";
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
// Once all of a bracket match's players are in its room, the bot checks their slots this
// long after the last join (and once it has nothing left to send), and moves anyone still
// out of place: up to REPOSITION_TRIES times, REPOSITION_EVERY apart.
const REPOSITION_SETTLE: Duration = Duration::from_secs(3);
const REPOSITION_EVERY: Duration = Duration::from_secs(10);
const REPOSITION_TRIES: u8 = 3;
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
    // When BanchoBot's answer is due; while queued, when a free bot is.
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
    // A finished match's room: the close the room was told (close_at is GRACE after it),
    // whether its last reminder went out, and the close the auto referee's closing line
    // told as it went out (it can go out before the match is saved as concluded).
    close_ends: Option<OffsetDateTime>,
    close_warned: bool,
    close_told: Option<OffsetDateTime>,
    // Who joined under a name no member has, waiting for !mp settings to give their id.
    unknown: HashSet<String>,
    // Who the bot sent !mp addref for, by normalize_name.
    refs_asked: HashSet<String>,
    // The score the tiebreaker was last put on for.
    tiebreaker_done: Option<String>,
    // When to check the players' slots again once everyone's in (None: not until someone
    // joins), and how many times they were moved since the last join.
    reposition_at: Option<Instant>,
    reposition_tries: u8,
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
    // Rooms asked for while every bot with room to spare was busy making one (a bot makes
    // one at a time): each is made as soon as a bot is free.
    queued: VecDeque<Making>,
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
        queued: VecDeque::new(),
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

fn is_bancho_bot(name: &str) -> bool {
    name.eq_ignore_ascii_case(BANCHO_BOT)
}

fn run_dirty(runs: &mut HashMap<Uuid, Run>, id: Uuid) {
    if let Some(run) = runs.get_mut(&id) {
        run.dirty = true;
    }
}
