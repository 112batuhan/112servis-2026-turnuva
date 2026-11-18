//! The site's own osu! accounts on Bancho, which run the multiplayer rooms: they make
//! each room, log every line said in it, read it into the room's state, count the
//! score and save it, and do what a room needs doing by itself (placing players,
//! kicking strangers, adding referees, reading mods, the tiebreaker). There is one copy
//! of every room, here, and every page reads it (view::LiveRoom over the live socket).
//!
//! Hosts add the accounts on /admin/bancho; every active one stays connected
//! (bot.rs), and the rooms are spread across them (manager.rs): each account can have
//! ROOM_LIMIT rooms open, counting the ones it was only made a referee of, so a bot is
//! never added to another bot's room and the site makes no more rooms than they can
//! hold. Referees and hosts don't join the rooms: they run them from the referee page,
//! and talk in them through the room's bot (RoomCommand::Say). Unless a referee switches
//! a match to manual, the room's bot also referees it by itself (autoref.rs), opening its
//! room 10 minutes before its time.

pub mod autoref;
pub mod bot;
pub mod commands;
pub mod context;
pub mod irc;
mod manager;
pub mod room;
pub mod tally;
pub mod view;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tokio::sync::{mpsc, oneshot};
use uuid::Uuid;

use crate::{
    error::{AppError, BanchoError},
    osu_api::matches::MatchGame,
    AppState,
};

use self::{room::SlotTeam, view::LiveRoom};

// How many rooms an osu! account can have open at once on Bancho, counting the ones it
// was made a referee of.
pub const ROOM_LIMIT: usize = 4;

// What a referee page asks the room's bot to do (POST /api/matches/:id/room/command).
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RoomCommand {
    // Team mode, score mode, size and lock again, then invites everyone.
    SetUp,
    // !mp settings: reads the room anew.
    Settings,
    Invite {
        osu_id: i64,
    },
    InviteAll,
    // Moves whoever is in `slot` to slot `to` and onto `team` (either left out to keep it).
    Move {
        slot: i32,
        to: Option<i32>,
        team: Option<SlotTeam>,
    },
    // Puts every bracket player in their slot and their team's colour.
    PlaceAll,
    Kick {
        slot: i32,
    },
    // Kicks everyone who isn't a player of the match, a host, a referee or a streamer.
    KickOutsiders,
    Lock {
        locked: bool,
    },
    Timer {
        seconds: u32,
    },
    AbortTimer,
    Start {
        seconds: u32,
    },
    Abort,
    Close,
    // Sets a pool map and its mods.
    Map {
        map_id: Uuid,
    },
    // Tells the room the score and whose pick is next.
    AnnounceScore,
    // Makes a referee or streamer one of the room's referees, or takes it back.
    AddRef {
        username: String,
    },
    RemoveRef {
        username: String,
    },
    // The bot tries the room's channel again (a room made elsewhere, once its maker
    // added the bot).
    Rejoin,
    // Says something in the room through its bot: referees and hosts run the room from
    // the site and don't join it. Signed with their name, unless the room's bot is their
    // own account; an !mp command goes as it is.
    Say {
        text: String,
        // Filled in by the handler: who's saying it, by osu! username.
        #[serde(skip)]
        by: String,
    },
}

// A bot as the bots page shows it, beside its account.
#[derive(Debug, Clone, Serialize)]
pub struct BotStatus {
    pub id: Uuid,
    pub state: BotState,
    // Why it's offline, when it isn't by choice.
    pub error: Option<String>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub online_since: Option<OffsetDateTime>,
    // Rooms that count against its limit.
    pub rooms: Vec<BotRoom>,
    // Lines waiting to go out, and sent in the last window.
    pub queued: usize,
    pub recent: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BotState {
    Online,
    Connecting,
    // Turned off on the bots page.
    Off,
    // Bancho refused its IRC password.
    LoginFailed,
    // Bancho said it has as many rooms as it can open.
    Full,
}

#[derive(Debug, Clone, Serialize)]
pub struct BotRoom {
    pub mp_id: i64,
    pub match_id: Uuid,
    // "W3", "Lobi B"; None for a match that's over.
    pub label: Option<String>,
    // The bot made it, rather than being added to it.
    pub made: bool,
    pub access: view::RoomAccess,
}

// The pace the bots keep, for the bots page.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct BotLimits {
    pub rooms: usize,
    pub lines: usize,
    pub window_secs: u64,
}

pub const LIMITS: BotLimits = BotLimits {
    rooms: ROOM_LIMIT,
    lines: bot::SEND_LIMIT,
    window_secs: bot::SEND_WINDOW.as_secs(),
};

enum Request {
    View {
        match_id: Uuid,
        reply: oneshot::Sender<Option<LiveRoom>>,
    },
    CreateRoom {
        match_id: Uuid,
        reply: oneshot::Sender<Result<i64, AppError>>,
    },
    AssignBot {
        reply: oneshot::Sender<Option<Uuid>>,
    },
    Command {
        match_id: Uuid,
        command: RoomCommand,
        reply: oneshot::Sender<Result<(), AppError>>,
    },
    SaveScore {
        match_id: Uuid,
        reply: oneshot::Sender<Result<(), AppError>>,
    },
    OsuGames {
        match_id: Uuid,
        at: OffsetDateTime,
        games: HashMap<i64, Vec<MatchGame>>,
        // The referee asked for it: saved even if it's the first reading.
        force: bool,
    },
    // The auto referee's osu! read is done: `confirmed` when it had the latest map.
    AutorefOsu {
        match_id: Uuid,
        confirmed: bool,
    },
    // A warmup beatmap's length, for the auto referee.
    AutorefLength {
        match_id: Uuid,
        beatmap_id: i64,
        seconds: i32,
    },
    BotsChanged,
    Resync,
    Status {
        reply: oneshot::Sender<Vec<BotStatus>>,
    },
}

// The handle every handler holds. Cheap to clone.
#[derive(Clone)]
pub struct Bancho {
    tx: mpsc::UnboundedSender<Request>,
}

const GONE: BanchoError = BanchoError::Restarting;

impl Bancho {
    // The handle, and what `spawn` starts the manager with.
    pub fn new() -> (Self, BanchoInbox) {
        let (tx, rx) = mpsc::unbounded_channel();
        (Bancho { tx }, BanchoInbox(rx))
    }

    fn send(&self, request: Request) {
        let _ = self.tx.send(request);
    }

    async fn ask<T>(&self, request: impl FnOnce(oneshot::Sender<T>) -> Request) -> Option<T> {
        let (reply, rx) = oneshot::channel();
        self.send(request(reply));
        rx.await.ok()
    }

    // The match's room as the bots run it; None for a match they don't (it has no room,
    // or it's over).
    pub async fn view(&self, match_id: Uuid) -> Option<LiveRoom> {
        self.ask(|reply| Request::View { match_id, reply })
            .await
            .flatten()
    }

    // Makes a room for the match with the least busy bot that has room for one, sets it
    // up and adds it to the match. Returns its id.
    pub async fn create_room(&self, match_id: Uuid) -> Result<i64, AppError> {
        self.ask(|reply| Request::CreateRoom { match_id, reply })
            .await
            .unwrap_or(Err(GONE.into()))
    }

    // The bot a room made elsewhere is handed to: the least busy one with room for it.
    pub async fn assign_bot(&self) -> Option<Uuid> {
        self.ask(|reply| Request::AssignBot { reply })
            .await
            .flatten()
    }

    pub async fn command(&self, match_id: Uuid, command: RoomCommand) -> Result<(), AppError> {
        self.ask(|reply| Request::Command {
            match_id,
            command,
            reply,
        })
        .await
        .unwrap_or(Err(GONE.into()))
    }

    // Saves the score as the bots count it, now; returns once it's saved.
    pub async fn save_score(&self, match_id: Uuid) -> Result<(), AppError> {
        self.ask(|reply| Request::SaveScore { match_id, reply })
            .await
            .unwrap_or(Err(GONE.into()))
    }

    // osu!'s record of the match's rooms, read as of `at`: it takes the place of the
    // chat's maps.
    pub fn osu_games(
        &self,
        match_id: Uuid,
        at: OffsetDateTime,
        games: HashMap<i64, Vec<MatchGame>>,
    ) {
        self.send(Request::OsuGames {
            match_id,
            at,
            games,
            force: true,
        });
    }

    // A bot account was added, changed, switched or deleted.
    pub fn bots_changed(&self) {
        self.send(Request::BotsChanged);
    }

    // Reads every running match again (the staff changed, say).
    pub fn resync(&self) {
        self.send(Request::Resync);
    }

    pub async fn status(&self) -> Vec<BotStatus> {
        self.ask(|reply| Request::Status { reply })
            .await
            .unwrap_or_default()
    }
}

pub use manager::{backfill_lobbies, read_osu_games, static_view};

pub struct BanchoInbox(mpsc::UnboundedReceiver<Request>);

// Starts the manager and the active bots.
pub fn spawn(state: &AppState, inbox: BanchoInbox) {
    let state = state.clone();
    let inbox = crate::supervise::shared(inbox.0);
    crate::supervise::spawn("bancho manager", move || {
        manager::run(state.clone(), inbox.clone())
    });
}
