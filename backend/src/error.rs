//! The errors the API answers with.
//!
//! Handlers return `Result<_, AppError>`. Every error a client can get is a variant of one
//! of the grouped enums below (`TeamError::NameTaken`, `LobbyError::Full`…): the variant is
//! the code, carries its English message and has its HTTP status (`status()`). `?` and
//! `.into()` turn any of them into an AppError. The client gets
//! `{ "group": "team", "code": "name_taken", "message": "that team name is taken" }`; the
//! frontend shows each code in Turkish (`ERROR_TEXT` in frontend/src/api/errors.ts, which
//! `errors.test.ts` checks against the enums here). The English message is for logs and for
//! anything that doesn't know the code.
//!
//! AppError's other variants keep an internal cause for the log (5xx only) and answer with
//! a general code.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Code(#[from] ErrorCode),

    #[error("osu! token exchange failed: {0}")]
    OsuTokenExchange(String),

    #[error("could not reach osu!")]
    OsuProfileRequest(#[source] reqwest::Error),

    #[error("could not read osu! profile")]
    OsuProfileDecode(#[source] reqwest::Error),

    #[error("discord token exchange failed: {0}")]
    DiscordTokenExchange(String),

    #[error("could not reach Discord")]
    DiscordProfileRequest(#[source] reqwest::Error),

    #[error("could not read Discord profile")]
    DiscordProfileDecode(#[source] reqwest::Error),

    #[error("could not reach osu! API")]
    OsuApiRequest(#[source] reqwest::Error),

    #[error("could not read osu! API response")]
    OsuApiDecode(#[source] reqwest::Error),

    #[error("database error")]
    Database(#[from] sqlx::Error),

    #[error("could not serialize a response")]
    Serialize(#[from] serde_json::Error),
}

impl AppError {
    // The code the client gets.
    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::Code(code) => *code,
            AppError::OsuTokenExchange(_)
            | AppError::OsuProfileRequest(_)
            | AppError::OsuProfileDecode(_) => AuthError::OsuLoginFailed.into(),
            AppError::DiscordTokenExchange(_)
            | AppError::DiscordProfileRequest(_)
            | AppError::DiscordProfileDecode(_) => AuthError::DiscordLinkFailed.into(),
            AppError::OsuApiRequest(_) => GeneralError::OsuApiUnreachable.into(),
            AppError::OsuApiDecode(_) => GeneralError::OsuApiBadResponse.into(),
            AppError::Database(_) | AppError::Serialize(_) => GeneralError::Internal.into(),
        }
    }
}

#[derive(Serialize)]
struct ErrorBody {
    #[serde(flatten)]
    code: ErrorCode,
    message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let code = self.code();
        let status = code.status();

        // Client only sees the code and its generic message; the real cause goes to the
        // logs.
        if status.is_server_error() {
            tracing::error!(error = ?self, "{self}");
        }

        let body = ErrorBody {
            code,
            message: code.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

// Formats an error together with its full source chain. Useful for otherwise
// opaque transport failures like oauth2's "Request failed", whose real cause
// (connection reset, DNS, TLS, ...) is only in the source.
pub fn chain(err: &dyn std::error::Error) -> String {
    let mut out = err.to_string();
    let mut source = err.source();
    while let Some(e) = source {
        out.push_str(": ");
        out.push_str(&e.to_string());
        source = e.source();
    }
    out
}

/// Every error a client can get, by group. Serialized as `"group": "team", "code": "…"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(tag = "group", content = "code", rename_all = "snake_case")]
pub enum ErrorCode {
    #[error(transparent)]
    General(#[from] GeneralError),
    #[error(transparent)]
    Auth(#[from] AuthError),
    #[error(transparent)]
    Team(#[from] TeamError),
    #[error(transparent)]
    Pool(#[from] PoolError),
    #[error(transparent)]
    Bracket(#[from] BracketError),
    #[error(transparent)]
    Match(#[from] MatchError),
    #[error(transparent)]
    Schedule(#[from] ScheduleError),
    #[error(transparent)]
    Staff(#[from] StaffError),
    #[error(transparent)]
    Lobby(#[from] LobbyError),
    #[error(transparent)]
    Bancho(#[from] BanchoError),
    #[error(transparent)]
    Room(#[from] RoomError),
    #[error(transparent)]
    Discord(#[from] DiscordError),
    #[error(transparent)]
    Pickem(#[from] PickemError),
    #[error(transparent)]
    Arg(#[from] ArgError),
    #[error(transparent)]
    Rules(#[from] RulesError),
}

impl ErrorCode {
    pub fn status(self) -> StatusCode {
        match self {
            ErrorCode::General(e) => e.status(),
            ErrorCode::Auth(e) => e.status(),
            ErrorCode::Team(e) => e.status(),
            ErrorCode::Pool(e) => e.status(),
            ErrorCode::Bracket(e) => e.status(),
            ErrorCode::Match(e) => e.status(),
            ErrorCode::Schedule(e) => e.status(),
            ErrorCode::Staff(e) => e.status(),
            ErrorCode::Lobby(e) => e.status(),
            ErrorCode::Bancho(e) => e.status(),
            ErrorCode::Room(e) => e.status(),
            ErrorCode::Discord(e) => e.status(),
            ErrorCode::Pickem(e) => e.status(),
            ErrorCode::Arg(e) => e.status(),
            ErrorCode::Rules(e) => e.status(),
        }
    }
}

// `TeamError::NameTaken.into()` and `?` on a group's error give an AppError.
macro_rules! into_app_error {
    ($($group:ident),*) => {
        $(impl From<$group> for AppError {
            fn from(err: $group) -> Self {
                AppError::Code(err.into())
            }
        })*
    };
}

into_app_error!(
    GeneralError,
    AuthError,
    TeamError,
    PoolError,
    BracketError,
    MatchError,
    ScheduleError,
    StaffError,
    LobbyError,
    BanchoError,
    RoomError,
    DiscordError,
    PickemError,
    ArgError,
    RulesError
);

/// Not about any one feature, or a failure on our side or osu!'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GeneralError {
    #[error("not found")]
    NotFound,
    #[error("beatmap not found")]
    BeatmapNotFound,
    #[error("could not reach osu! API")]
    OsuApiUnreachable,
    #[error("could not read osu! API response")]
    OsuApiBadResponse,
    #[error("internal server error")]
    Internal,
}

impl GeneralError {
    fn status(self) -> StatusCode {
        match self {
            Self::NotFound | Self::BeatmapNotFound => StatusCode::NOT_FOUND,
            Self::Internal => StatusCode::INTERNAL_SERVER_ERROR,
            Self::OsuApiUnreachable | Self::OsuApiBadResponse => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Signing in, linking Discord, roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthError {
    #[error("not authenticated")]
    Unauthenticated,
    #[error("insufficient permissions")]
    Forbidden,
    #[error("link your Discord account first")]
    DiscordRequired,
    #[error("rejoin the tournament's Discord server first")]
    NotInDiscordServer,
    #[error("invalid or expired login attempt")]
    InvalidLogin,
    #[error("could not complete osu! login")]
    OsuLoginFailed,
    #[error("could not link Discord")]
    DiscordLinkFailed,
    #[error("you can't remove your own host role")]
    OwnHostRole,
    #[error("Sign in with a real osu! account to seed a live test for it.")]
    DevLiveNeedsRealAccount,
}

impl AuthError {
    fn status(self) -> StatusCode {
        match self {
            Self::InvalidLogin | Self::OwnHostRole | Self::DevLiveNeedsRealAccount => {
                StatusCode::BAD_REQUEST
            }
            Self::Unauthenticated => StatusCode::UNAUTHORIZED,
            Self::Forbidden | Self::DiscordRequired => StatusCode::FORBIDDEN,
            Self::NotInDiscordServer => StatusCode::CONFLICT,
            Self::OsuLoginFailed | Self::DiscordLinkFailed => StatusCode::BAD_GATEWAY,
        }
    }
}

/// Teams, invites, the rank limit, banners.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamError {
    #[error("you aren't in a team")]
    NotInTeam,
    #[error("you are already in a team")]
    AlreadyInTeam,
    #[error("that player is already in a team")]
    PlayerInTeam,
    #[error("one of you is already in a team")]
    SomeoneInTeam,
    #[error("staff can't join teams")]
    StaffCantPlay,
    #[error("both players need a global rank")]
    NeedsGlobalRank,
    #[error("this team would be over the rank limit")]
    OverRankLimit,
    #[error("a rank update is already running")]
    RankUpdateRunning,
    #[error("teams are frozen")]
    Frozen,
    #[error("team names must be 1 to 32 characters")]
    NameLength,
    #[error("team names can't contain control characters")]
    NameCharacters,
    #[error("that team name is taken")]
    NameTaken,
    #[error("you can't invite yourself")]
    InviteSelf,
    #[error("invite already sent")]
    InviteAlreadySent,
    #[error("they already invited you, accept their invite instead")]
    TheyInvitedYou,
    #[error("banners must be PNG, JPEG, GIF or WebP images")]
    BadBanner,
}

impl TeamError {
    fn status(self) -> StatusCode {
        match self {
            Self::NotInTeam
            | Self::StaffCantPlay
            | Self::NeedsGlobalRank
            | Self::OverRankLimit
            | Self::NameLength
            | Self::NameCharacters
            | Self::InviteSelf
            | Self::BadBanner => StatusCode::BAD_REQUEST,
            Self::AlreadyInTeam
            | Self::PlayerInTeam
            | Self::SomeoneInTeam
            | Self::RankUpdateRunning
            | Self::Frozen
            | Self::NameTaken
            | Self::InviteAlreadySent
            | Self::TheyInvitedYou => StatusCode::CONFLICT,
        }
    }
}

/// Stages and their map pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PoolError {
    #[error("a name is required")]
    NameRequired,
    #[error("that stage doesn't exist")]
    UnknownStage,
    #[error("the qualifier stage just changed, try again")]
    QualifierStageChanged,
    #[error("best of must be an odd number from 1 to 25")]
    BadBestOf,
    #[error("the order needs at least one ban or pick")]
    DraftOrderEmpty,
    #[error("each step is p1, p2, b1 or b2, and * only at the end")]
    DraftOrderStep,
    #[error("the order has more than 40 steps")]
    DraftOrderTooLong,
    #[error("the pack link isn't a valid address")]
    BadPackLink,
    #[error("the pack link must start with http:// or https://")]
    PackLinkScheme,
    #[error("the pack link is too long")]
    PackLinkTooLong,
}

impl PoolError {
    fn status(self) -> StatusCode {
        match self {
            Self::NameRequired
            | Self::UnknownStage
            | Self::BadBestOf
            | Self::DraftOrderEmpty
            | Self::DraftOrderStep
            | Self::DraftOrderTooLong
            | Self::BadPackLink
            | Self::PackLinkScheme
            | Self::PackLinkTooLong => StatusCode::BAD_REQUEST,
            Self::QualifierStageChanged => StatusCode::CONFLICT,
        }
    }
}

/// The bracket's shape: match numbers, teams and links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BracketError {
    #[error("match numbers start at 1")]
    BadMatchNumber,
    #[error("another match already has that number")]
    MatchNumberTaken,
    #[error("that team doesn't exist")]
    UnknownTeam,
    #[error("a team can't play itself")]
    TeamPlaysItself,
    #[error("slot must be 1 or 2")]
    BadSlot,
    #[error("that slot already gets a team from another match")]
    SlotHasSource,
    #[error("a match can't lead into itself")]
    LinkToItself,
    #[error("that link would make the bracket loop back to this match")]
    Loop,
}

impl BracketError {
    fn status(self) -> StatusCode {
        match self {
            Self::BadMatchNumber
            | Self::UnknownTeam
            | Self::TeamPlaysItself
            | Self::BadSlot
            | Self::LinkToItself
            | Self::Loop => StatusCode::BAD_REQUEST,
            Self::MatchNumberTaken | Self::SlotHasSource => StatusCode::CONFLICT,
        }
    }
}

/// A match's state, results, bans and picks, and notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchError {
    #[error("this match changed, reload and try again")]
    Changed,
    #[error("this match is concluded")]
    Concluded,
    #[error("this match is concluded, reopen it to change its results")]
    ConcludedResults,
    #[error("this match is concluded, reopen it to change the score")]
    ConcludedScore,
    #[error("this match is concluded, reopen it before changing its teams or links")]
    ConcludedTeams,
    #[error("a match this one leads into is already concluded, reopen it first")]
    NextConcluded,
    #[error("this match has no room yet")]
    NeedsRoom,
    #[error("a concluded match needs a room, reopen it first")]
    ConcludedNeedsRoom,
    #[error("this match needs both teams first")]
    NeedsBothTeams,
    #[error("enter a score with a winner first")]
    NeedsWinner,
    #[error("that team doesn't play in this match")]
    TeamNotInMatch,
    #[error("that player isn't in this match")]
    PlayerNotInMatch,
    #[error("scores must be between 0 and 99")]
    BadScore,
    #[error("unknown score mode")]
    BadScoreMode,
    #[error("too many map results")]
    TooManyResults,
    #[error("every map result needs its map")]
    ResultNeedsMap,
    #[error("map scores can't be negative")]
    NegativeMapScore,
    #[error("bad player scores on a map")]
    BadMapScores,
    #[error("bad mods on a map")]
    BadMapMods,
    #[error("that map isn't in this match's pool")]
    MapNotInPool,
    #[error("that isn't a map of this match")]
    NotAMatchMap,
    #[error("that map is banned")]
    MapBanned,
    #[error("that map has been picked")]
    MapPicked,
    #[error("the note can be at most 200 characters")]
    NoteTooLong,
    #[error("notes can be at most 2000 characters")]
    NotesTooLong,
}

impl MatchError {
    fn status(self) -> StatusCode {
        match self {
            Self::ConcludedNeedsRoom
            | Self::NeedsBothTeams
            | Self::NeedsWinner
            | Self::TeamNotInMatch
            | Self::PlayerNotInMatch
            | Self::BadScore
            | Self::BadScoreMode
            | Self::TooManyResults
            | Self::ResultNeedsMap
            | Self::NegativeMapScore
            | Self::BadMapScores
            | Self::BadMapMods
            | Self::MapNotInPool
            | Self::NotAMatchMap
            | Self::NoteTooLong
            | Self::NotesTooLong => StatusCode::BAD_REQUEST,
            Self::Changed
            | Self::Concluded
            | Self::ConcludedResults
            | Self::ConcludedScore
            | Self::ConcludedTeams
            | Self::NextConcluded
            | Self::NeedsRoom
            | Self::MapBanned
            | Self::MapPicked => StatusCode::CONFLICT,
        }
    }
}

/// Reschedule proposals and the notifications that answer them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleError {
    #[error("this match has started, only the staff can move it now")]
    MatchStarted,
    #[error("this match doesn't have its opponent yet")]
    NoOpponent,
    #[error("pick a time in the future")]
    TimeInPast,
    #[error("that time has already passed")]
    TimePassed,
    #[error("the match is already at that time")]
    SameTime,
    #[error("your team already proposed that time")]
    AlreadyProposed,
    #[error("they already proposed that time, accept theirs instead")]
    TheyProposedTime,
    #[error("your team proposed this time, the other team has to accept it")]
    OwnProposal,
    #[error("your team has too many proposals for this match, cancel some first")]
    TooManyProposals,
    #[error("the match's time has changed since")]
    TimeChanged,
    #[error("the match has been moved again since, answer the newer notification")]
    MovedAgain,
    #[error("you already answered this")]
    AlreadyAnswered,
    #[error("you're no longer on this match")]
    NoLongerOnMatch,
}

impl ScheduleError {
    fn status(self) -> StatusCode {
        match self {
            Self::TimeInPast | Self::SameTime | Self::OwnProposal => StatusCode::BAD_REQUEST,
            Self::MatchStarted
            | Self::NoOpponent
            | Self::TimePassed
            | Self::AlreadyProposed
            | Self::TheyProposedTime
            | Self::TooManyProposals
            | Self::TimeChanged
            | Self::MovedAgain
            | Self::AlreadyAnswered
            | Self::NoLongerOnMatch => StatusCode::CONFLICT,
        }
    }
}

/// Taking a match as its referee or streamer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StaffError {
    #[error("only referees can referee")]
    NotAReferee,
    #[error("this match already has a referee")]
    HasReferee,
    #[error("only streamers can stream")]
    NotAStreamer,
    #[error("you can't stream a match you play in")]
    StreamOwnMatch,
    #[error("this match already has a streamer")]
    HasStreamer,
}

impl StaffError {
    fn status(self) -> StatusCode {
        match self {
            Self::NotAReferee | Self::NotAStreamer | Self::StreamOwnMatch => {
                StatusCode::BAD_REQUEST
            }
            Self::HasReferee | Self::HasStreamer => StatusCode::CONFLICT,
        }
    }
}

/// Qualifier lobbies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LobbyError {
    #[error("that lobby is closed")]
    Closed,
    #[error("your lobby has already started")]
    Started,
    #[error("that lobby is full")]
    Full,
}

impl LobbyError {
    fn status(self) -> StatusCode {
        match self {
            Self::Closed | Self::Started | Self::Full => StatusCode::CONFLICT,
        }
    }
}

/// The Bancho bots and their manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BanchoError {
    #[error("that isn't an osu! username")]
    BadUsername,
    #[error("that isn't an IRC password")]
    BadIrcPassword,
    #[error("that account is already a bot")]
    AlreadyABot,
    #[error("BOT_SECRET_KEY isn't set on the server")]
    BotSecretMissing,
    #[error("no bot is connected to Bancho")]
    NoBotConnected,
    #[error("every bot already has as many rooms open as Bancho allows (4 each)")]
    BotsFull,
    #[error("the bot lost its connection to Bancho")]
    BotDisconnected,
    #[error("the bot was turned off")]
    BotTurnedOff,
    #[error("BanchoBot didn't answer; look at the bot's chat")]
    BanchobotNoAnswer,
    #[error("the room manager is restarting, try again")]
    Restarting,
}

impl BanchoError {
    fn status(self) -> StatusCode {
        match self {
            Self::BadUsername | Self::BadIrcPassword | Self::BotSecretMissing => {
                StatusCode::BAD_REQUEST
            }
            Self::AlreadyABot
            | Self::NoBotConnected
            | Self::BotsFull
            | Self::BotDisconnected
            | Self::BotTurnedOff
            | Self::BanchobotNoAnswer
            | Self::Restarting => StatusCode::CONFLICT,
        }
    }
}

/// A match's osu! multiplayer room: linking it, its bot, its referees, its chat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomError {
    #[error("that isn't an osu! multiplayer link")]
    BadMpLink,
    #[error("that room is already linked to a match")]
    Taken,
    #[error("this room is closed")]
    Closed,
    #[error("the bots don't run this match")]
    MatchNotRun,
    #[error("no bot runs this room; open a new room")]
    WithoutBot,
    #[error("the room's bot isn't connected to Bancho; open a new room")]
    BotOffline,
    #[error("the room's bot isn't in the room yet")]
    BotNotInRoom,
    #[error("slots go from 1 to 16")]
    BadSlot,
    #[error("nobody is in that slot")]
    SlotEmpty,
    #[error("the staff list isn't loaded yet")]
    StaffNotLoaded,
    #[error("only referees, streamers and commentators can be added as referees")]
    CantBeReferee,
    #[error("a player of the match can't be one of its room's referees")]
    PlayerCantReferee,
    #[error("a bot is never added to another bot's room")]
    BotInBotRoom,
    #[error("they aren't a referee of this room")]
    NotReferee,
    #[error("add and remove the room's referees from its settings")]
    RefsFromSettings,
    #[error("write something first")]
    EmptyMessage,
}

impl RoomError {
    fn status(self) -> StatusCode {
        match self {
            Self::BadMpLink
            | Self::BadSlot
            | Self::SlotEmpty
            | Self::CantBeReferee
            | Self::PlayerCantReferee
            | Self::BotInBotRoom
            | Self::NotReferee
            | Self::RefsFromSettings
            | Self::EmptyMessage => StatusCode::BAD_REQUEST,
            Self::Taken
            | Self::Closed
            | Self::MatchNotRun
            | Self::WithoutBot
            | Self::BotOffline
            | Self::BotNotInRoom
            | Self::StaffNotLoaded => StatusCode::CONFLICT,
        }
    }
}

/// The Discord bot and pinging players through it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscordError {
    #[error("the Discord bot isn't set up")]
    BotMissing,
    #[error("the Discord bot is off")]
    BotOff,
    #[error("this match has no Discord channel yet")]
    NoChannel,
    #[error("that player isn't in the Discord server")]
    PlayerNotInServer,
    #[error("that player was just pinged, wait a little")]
    PingCooldown,
}

impl DiscordError {
    fn status(self) -> StatusCode {
        match self {
            Self::BotMissing
            | Self::BotOff
            | Self::NoChannel
            | Self::PlayerNotInServer
            | Self::PingCooldown => StatusCode::CONFLICT,
        }
    }
}

/// The pick'em.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PickemError {
    #[error("this stage's pick'em isn't open")]
    Closed,
    #[error("this match can't be predicted")]
    CantPredict,
    #[error("that score isn't possible in this match")]
    ImpossibleScore,
}

impl PickemError {
    fn status(self) -> StatusCode {
        match self {
            Self::ImpossibleScore => StatusCode::BAD_REQUEST,
            Self::Closed | Self::CantPredict => StatusCode::CONFLICT,
        }
    }
}

/// The hidden /kupa puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArgError {
    #[error("invalid guess")]
    InvalidGuess,
    #[error("too many guesses; wait 10 minutes and try again")]
    TooManyGuesses,
}

impl ArgError {
    fn status(self) -> StatusCode {
        match self {
            Self::InvalidGuess => StatusCode::BAD_REQUEST,
            Self::TooManyGuesses => StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

/// The rules page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RulesError {
    #[error("the rules are too long")]
    TooLong,
    #[error("someone else saved the rules after you opened them; reload to see their version")]
    Changed,
}

impl RulesError {
    fn status(self) -> StatusCode {
        match self {
            Self::TooLong => StatusCode::BAD_REQUEST,
            Self::Changed => StatusCode::CONFLICT,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_group_code_and_message() {
        let body = ErrorBody {
            code: TeamError::NameTaken.into(),
            message: TeamError::NameTaken.to_string(),
        };
        assert_eq!(
            serde_json::to_string(&body).unwrap(),
            r#"{"group":"team","code":"name_taken","message":"that team name is taken"}"#
        );
    }

    #[test]
    fn status_comes_from_the_code() {
        let err = AppError::from(LobbyError::Full);
        assert_eq!(err.code().status(), StatusCode::CONFLICT);
        assert_eq!(
            AppError::from(PoolError::BadBestOf).code().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            AppError::Database(sqlx::Error::RowNotFound).code(),
            ErrorCode::General(GeneralError::Internal)
        );
    }
}
