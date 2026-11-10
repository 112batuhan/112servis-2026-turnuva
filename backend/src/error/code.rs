//! Every error the API can answer with, as a code. It's serialized in snake_case
//! (`LobbyFull` is `"lobby_full"`), which is what the frontend goes by: it shows each in
//! Turkish (frontend/src/api/errors.ts, whose `ERROR_CODES` lists the same variants).
//! The English message is for logs and for anything that doesn't know the code.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthenticated,
    Forbidden,
    DiscordRequired,
    InvalidLogin,
    OsuLoginFailed,
    DiscordLinkFailed,
    NotFound,
    BeatmapNotFound,
    OsuApiUnreachable,
    OsuApiBadResponse,
    Internal,

    AlreadyABot,
    AlreadyAnswered,
    AlreadyInTeam,
    AlreadyProposed,
    BadBannerType,
    BadBestOf,
    BadIrcPassword,
    BadMapMods,
    BadMapScores,
    BadMatchNumber,
    BadMpLink,
    BadPackLink,
    BadRoomSlot,
    BadScore,
    BadScoreMode,
    BadSlot,
    BadUsername,
    BanchoRestarting,
    BanchobotNoAnswer,
    BotDisconnected,
    BotInBotRoom,
    BotNotInRoom,
    BotSecretMissing,
    BotTurnedOff,
    BotsFull,
    BracketLoop,
    CantBeRoomReferee,
    CantPredict,
    ConcludedNeedsRoom,
    DevLiveNeedsRealAccount,
    DiscordBotMissing,
    DiscordBotOff,
    DraftOrderEmpty,
    DraftOrderStep,
    DraftOrderTooLong,
    EmptyMessage,
    HasReferee,
    HasStreamer,
    ImpossibleScore,
    InvalidGuess,
    InviteAlreadySent,
    InviteSelf,
    LinkToItself,
    LobbyClosed,
    LobbyFull,
    LobbyStarted,
    MapBanned,
    MapNotInPool,
    MapPicked,
    MatchChanged,
    MatchConcluded,
    MatchConcludedResults,
    MatchConcludedScore,
    MatchConcludedTeams,
    MatchNeedsRoom,
    MatchNotRun,
    MatchNumberTaken,
    MatchStarted,
    MovedAgain,
    NameRequired,
    NeedsBothTeams,
    NeedsGlobalRank,
    NeedsWinner,
    NegativeMapScore,
    NextMatchConcluded,
    NoBotConnected,
    NoDiscordChannel,
    NoLongerOnMatch,
    NoOpponent,
    NotAMatchMap,
    NotAReferee,
    NotAStreamer,
    NotInDiscordServer,
    NotInTeam,
    NotRoomReferee,
    NoteTooLong,
    NotesTooLong,
    OverRankLimit,
    OwnHostRole,
    OwnProposal,
    PackLinkScheme,
    PackLinkTooLong,
    PickemClosed,
    PingCooldown,
    PlayerCantReferee,
    PlayerInTeam,
    PlayerNotInMatch,
    PlayerNotInServer,
    QualifierStageChanged,
    RankUpdateRunning,
    RefsFromSettings,
    ResultNeedsMap,
    RoomBotOffline,
    RoomClosed,
    RoomTaken,
    RoomWithoutBot,
    RulesChanged,
    RulesTooLong,
    SameTime,
    SlotEmpty,
    SlotHasSource,
    SomeoneInTeam,
    StaffCantPlay,
    StaffNotLoaded,
    StreamOwnMatch,
    TeamNameCharacters,
    TeamNameLength,
    TeamNameTaken,
    TeamNotInMatch,
    TeamPlaysItself,
    TeamsFrozen,
    TheyInvitedYou,
    TheyProposedTime,
    TimeChanged,
    TimeInPast,
    TimePassed,
    TooManyGuesses,
    TooManyProposals,
    TooManyResults,
    UnknownStage,
    UnknownTeam,
}

impl ErrorCode {
    pub fn message(self) -> &'static str {
        match self {
            ErrorCode::Unauthenticated => "not authenticated",
            ErrorCode::Forbidden => "insufficient permissions",
            ErrorCode::DiscordRequired => "link your Discord account first",
            ErrorCode::InvalidLogin => "invalid or expired login attempt",
            ErrorCode::OsuLoginFailed => "could not complete osu! login",
            ErrorCode::DiscordLinkFailed => "could not link Discord",
            ErrorCode::NotFound => "not found",
            ErrorCode::BeatmapNotFound => "beatmap not found",
            ErrorCode::OsuApiUnreachable => "could not reach osu! API",
            ErrorCode::OsuApiBadResponse => "could not read osu! API response",
            ErrorCode::Internal => "internal server error",
            ErrorCode::AlreadyABot => "that account is already a bot",
            ErrorCode::AlreadyAnswered => "you already answered this",
            ErrorCode::AlreadyInTeam => "you are already in a team",
            ErrorCode::AlreadyProposed => "your team already proposed that time",
            ErrorCode::BadBannerType => "banners must be PNG, JPEG, GIF or WebP images",
            ErrorCode::BadBestOf => "best of must be an odd number from 1 to 25",
            ErrorCode::BadIrcPassword => "that isn't an IRC password",
            ErrorCode::BadMapMods => "bad mods on a map",
            ErrorCode::BadMapScores => "bad player scores on a map",
            ErrorCode::BadMatchNumber => "match numbers start at 1",
            ErrorCode::BadMpLink => "that isn't an osu! multiplayer link",
            ErrorCode::BadPackLink => "the pack link isn't a valid address",
            ErrorCode::BadRoomSlot => "slots go from 1 to 16",
            ErrorCode::BadScore => "scores must be between 0 and 99",
            ErrorCode::BadScoreMode => "unknown score mode",
            ErrorCode::BadSlot => "slot must be 1 or 2",
            ErrorCode::BadUsername => "that isn't an osu! username",
            ErrorCode::BanchoRestarting => "the room manager is restarting, try again",
            ErrorCode::BanchobotNoAnswer => "BanchoBot didn't answer; look at the bot's chat",
            ErrorCode::BotDisconnected => "the bot lost its connection to Bancho",
            ErrorCode::BotInBotRoom => "a bot is never added to another bot's room",
            ErrorCode::BotNotInRoom => "the room's bot isn't in the room yet",
            ErrorCode::BotSecretMissing => "BOT_SECRET_KEY isn't set on the server",
            ErrorCode::BotTurnedOff => "the bot was turned off",
            ErrorCode::BotsFull => {
                "every bot already has as many rooms open as Bancho allows (4 each)"
            }
            ErrorCode::BracketLoop => "that link would make the bracket loop back to this match",
            ErrorCode::CantBeRoomReferee => {
                "only referees, streamers and commentators can be added as referees"
            }
            ErrorCode::CantPredict => "this match can't be predicted",
            ErrorCode::ConcludedNeedsRoom => "a concluded match needs a room, reopen it first",
            ErrorCode::DevLiveNeedsRealAccount => {
                "Sign in with a real osu! account to seed a live test for it."
            }
            ErrorCode::DiscordBotMissing => "the Discord bot isn't set up",
            ErrorCode::DiscordBotOff => "the Discord bot is off",
            ErrorCode::DraftOrderEmpty => "the order needs at least one ban or pick",
            ErrorCode::DraftOrderStep => "each step is p1, p2, b1 or b2, and * only at the end",
            ErrorCode::DraftOrderTooLong => "the order has more than 40 steps",
            ErrorCode::EmptyMessage => "write something first",
            ErrorCode::HasReferee => "this match already has a referee",
            ErrorCode::HasStreamer => "this match already has a streamer",
            ErrorCode::ImpossibleScore => "that score isn't possible in this match",
            ErrorCode::InvalidGuess => "invalid guess",
            ErrorCode::InviteAlreadySent => "invite already sent",
            ErrorCode::InviteSelf => "you can't invite yourself",
            ErrorCode::LinkToItself => "a match can't lead into itself",
            ErrorCode::LobbyClosed => "that lobby is closed",
            ErrorCode::LobbyFull => "that lobby is full",
            ErrorCode::LobbyStarted => "your lobby has already started",
            ErrorCode::MapBanned => "that map is banned",
            ErrorCode::MapNotInPool => "that map isn't in this match's pool",
            ErrorCode::MapPicked => "that map has been picked",
            ErrorCode::MatchChanged => "this match changed, reload and try again",
            ErrorCode::MatchConcluded => "this match is concluded",
            ErrorCode::MatchConcludedResults => {
                "this match is concluded, reopen it to change its results"
            }
            ErrorCode::MatchConcludedScore => {
                "this match is concluded, reopen it to change the score"
            }
            ErrorCode::MatchConcludedTeams => {
                "this match is concluded, reopen it before changing its teams or links"
            }
            ErrorCode::MatchNeedsRoom => "this match has no room yet",
            ErrorCode::MatchNotRun => "the bots don't run this match",
            ErrorCode::MatchNumberTaken => "another match already has that number",
            ErrorCode::MatchStarted => "this match has started, only the staff can move it now",
            ErrorCode::MovedAgain => {
                "the match has been moved again since, answer the newer notification"
            }
            ErrorCode::NameRequired => "a name is required",
            ErrorCode::NeedsBothTeams => "this match needs both teams first",
            ErrorCode::NeedsGlobalRank => "both players need a global rank",
            ErrorCode::NeedsWinner => "enter a score with a winner first",
            ErrorCode::NegativeMapScore => "map scores can't be negative",
            ErrorCode::NextMatchConcluded => {
                "a match this one leads into is already concluded, reopen it first"
            }
            ErrorCode::NoBotConnected => "no bot is connected to Bancho",
            ErrorCode::NoDiscordChannel => "this match has no Discord channel yet",
            ErrorCode::NoLongerOnMatch => "you're no longer on this match",
            ErrorCode::NoOpponent => "this match doesn't have its opponent yet",
            ErrorCode::NotAMatchMap => "that isn't a map of this match",
            ErrorCode::NotAReferee => "only referees can referee",
            ErrorCode::NotAStreamer => "only streamers can stream",
            ErrorCode::NotInDiscordServer => "rejoin the tournament's Discord server first",
            ErrorCode::NotInTeam => "you aren't in a team",
            ErrorCode::NotRoomReferee => "they aren't a referee of this room",
            ErrorCode::NoteTooLong => "the note can be at most 200 characters",
            ErrorCode::NotesTooLong => "notes can be at most 2000 characters",
            ErrorCode::OverRankLimit => "this team would be over the rank limit",
            ErrorCode::OwnHostRole => "you can't remove your own host role",
            ErrorCode::OwnProposal => {
                "your team proposed this time, the other team has to accept it"
            }
            ErrorCode::PackLinkScheme => "the pack link must start with http:// or https://",
            ErrorCode::PackLinkTooLong => "the pack link is too long",
            ErrorCode::PickemClosed => "this stage's pick'em isn't open",
            ErrorCode::PingCooldown => "that player was just pinged, wait a little",
            ErrorCode::PlayerCantReferee => {
                "a player of the match can't be one of its room's referees"
            }
            ErrorCode::PlayerInTeam => "that player is already in a team",
            ErrorCode::PlayerNotInMatch => "that player isn't in this match",
            ErrorCode::PlayerNotInServer => "that player isn't in the Discord server",
            ErrorCode::QualifierStageChanged => "the qualifier stage just changed, try again",
            ErrorCode::RankUpdateRunning => "a rank update is already running",
            ErrorCode::RefsFromSettings => "add and remove the room's referees from its settings",
            ErrorCode::ResultNeedsMap => "every map result needs its map",
            ErrorCode::RoomBotOffline => {
                "the room's bot isn't connected to Bancho; open a new room"
            }
            ErrorCode::RoomClosed => "this room is closed",
            ErrorCode::RoomTaken => "that room is already linked to a match",
            ErrorCode::RoomWithoutBot => "no bot runs this room; open a new room",
            ErrorCode::RulesChanged => {
                "someone else saved the rules after you opened them; reload to see their version"
            }
            ErrorCode::RulesTooLong => "the rules are too long",
            ErrorCode::SameTime => "the match is already at that time",
            ErrorCode::SlotEmpty => "nobody is in that slot",
            ErrorCode::SlotHasSource => "that slot already gets a team from another match",
            ErrorCode::SomeoneInTeam => "one of you is already in a team",
            ErrorCode::StaffCantPlay => "staff can't join teams",
            ErrorCode::StaffNotLoaded => "the staff list isn't loaded yet",
            ErrorCode::StreamOwnMatch => "you can't stream a match you play in",
            ErrorCode::TeamNameCharacters => "team names can't contain control characters",
            ErrorCode::TeamNameLength => "team names must be 1 to 32 characters",
            ErrorCode::TeamNameTaken => "that team name is taken",
            ErrorCode::TeamNotInMatch => "that team doesn't play in this match",
            ErrorCode::TeamPlaysItself => "a team can't play itself",
            ErrorCode::TeamsFrozen => "teams are frozen",
            ErrorCode::TheyInvitedYou => "they already invited you, accept their invite instead",
            ErrorCode::TheyProposedTime => "they already proposed that time, accept theirs instead",
            ErrorCode::TimeChanged => "the match's time has changed since",
            ErrorCode::TimeInPast => "pick a time in the future",
            ErrorCode::TimePassed => "that time has already passed",
            ErrorCode::TooManyGuesses => "too many guesses; wait 10 minutes and try again",
            ErrorCode::TooManyProposals => {
                "your team has too many proposals for this match, cancel some first"
            }
            ErrorCode::TooManyResults => "too many map results",
            ErrorCode::UnknownStage => "that stage doesn't exist",
            ErrorCode::UnknownTeam => "that team doesn't exist",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_in_snake_case() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::LobbyFull).unwrap(),
            r#""lobby_full""#
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::OsuApiBadResponse).unwrap(),
            r#""osu_api_bad_response""#
        );
    }
}
