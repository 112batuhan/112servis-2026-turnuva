// Response shapes returned by the Rust backend (see backend/src/db).

/** A staff role; a user holds any set of them, and none means a regular player. */
export type Role =
  | "host"
  | "map_pooler"
  | "referee"
  | "streamer"
  | "graphics_designer"
  | "pool_tester"
  | "custom_mapper"
  | "commentator";

export interface DiscordAccount {
  id: string;
  osu_user_id: string;
  discord_id: string;
  username: string;
  avatar: string | null;
  /** Whether it's in the tournament's Discord server; null until the bot has checked. */
  in_guild: boolean | null;
  created_at: string;
  updated_at: string;
}

export interface User {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  country_code: string | null;
  global_rank: number | null;
  country_rank: number | null;
  roles: Role[];
  created_at: string;
  updated_at: string;
  discord: DiscordAccount | null;
}

export interface AdminUser {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  country_code: string | null;
  global_rank: number | null;
  country_rank: number | null;
  roles: Role[];
  created_at: string;
  discord_id: string | null;
  discord_username: string | null;
  discord_avatar: string | null;
}

export interface Stage {
  id: string;
  name: string;
  position: number;
  published: boolean;
  /** The qualifier lobbies' map pool; at most one stage is. */
  is_qualifier: boolean;
  /** How many maps its bracket matches are played to (odd); null until the hosts set it. */
  best_of: number | null;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: string;
  stage_id: string;
  name: string;
  position: number;
  color: string;
}

export interface PublicSlot {
  id: string;
  category_id: string;
  position: number;
}

export interface Slot extends PublicSlot {
  editor_notes: string;
}

export interface PublicPoolMap {
  id: string;
  /** null = the map sits in the generic pool. */
  slot_id: string | null;
  position: number;
  mods: string;
  beatmap_id: number;
  beatmapset_id: number;
  artist: string;
  title: string;
  version: string;
  creator: string | null;
  star_rating: number;
  bpm: number;
  total_length: number;
  cs: number;
  ar: number;
  od: number;
  mode: string;
  cover_url: string | null;
  public_notes: string;
}

export interface PoolMap extends PublicPoolMap {
  editor_notes: string;
}

export type MapNoteField = "public_notes" | "editor_notes";

export interface StageDetail extends Stage {
  categories: Category[];
  slots: Slot[];
  /** The global generic pool plus every map placed in this stage. */
  maps: PoolMap[];
}

export interface PublicStageDetail extends Stage {
  categories: Category[];
  slots: PublicSlot[];
  maps: PublicPoolMap[];
}

export interface Player {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  country_code: string | null;
  global_rank: number | null;
}

/** GET /api/public/rules: the rules as osu! BBCode. `updated_at` is also the version a save is based on. */
export interface Rules {
  bbcode: string;
  updated_at: string;
  /** Who saved it last; null for the text the site started with. */
  updated_by: string | null;
}

/** GET /api/public/staff: everyone holding a role. Staff Discord accounts are public on purpose. */
export interface StaffMember {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  country_code: string | null;
  roles: Role[];
  discord_id: string | null;
  discord_username: string | null;
}

export type Eligibility = "eligible" | "not_player" | "in_team" | "unranked" | "over_limit";

export interface PlayerResult extends Player {
  eligibility: Eligibility;
  team_value: number | null;
  invite_sent: boolean;
  invite_received: boolean;
}

export interface Invite {
  id: string;
  created_at: string;
  team_name: string;
  /** The player on the other side of the invite. */
  player: Player;
  eligibility: Eligibility;
  team_value: number | null;
}

export interface Team<P extends Player = Player> {
  id: string;
  name: string;
  created_at: string;
  /** API path of the banner image, or null when the team has none. */
  banner_url: string | null;
  team_value: number | null;
  /** Set when a formed team's current ranks no longer fit the limit. */
  rank_warning: RankWarning | null;
  players: P[];
}

export type RankWarning = "over_limit" | "unranked";

export interface TeamList<P extends Player = Player> {
  teams: Team<P>[];
  frozen: boolean;
  limit: number;
  /** When every team member's rank was last refreshed from osu!. */
  ranks_updated_at: string | null;
}

export interface RankRefresh {
  updated: number;
  ranks_updated_at: string;
}

export interface AdminTeamPlayer extends Player {
  discord_id: string | null;
  discord_username: string | null;
  /** false once they've left the tournament's Discord server; null until the bot has checked. */
  discord_in_guild: boolean | null;
}

export interface TeamStatus {
  team: Team | null;
  /** Hosts froze the teams: players can't form, rename, re-banner or disband them. */
  frozen: boolean;
  global_rank: number | null;
  /** The best rank a teammate may have, or null when the caller is unranked. */
  min_partner_rank: number | null;
  limit: number;
  sent: Invite[];
  received: Invite[];
  /** The players of the caller's team who left the tournament's Discord server. */
  left_discord: string[];
}

/** A time one of a bracket match's teams proposed for it. */
export interface RescheduleProposal {
  id: string;
  starts_at: string;
  created_at: string;
  proposed_by: Player;
}

/** One of the caller's team's unconcluded bracket matches with both teams set. */
export interface RescheduleMatch {
  id: string;
  side: BracketSide;
  number: number;
  stage_name: string | null;
  /** null until the match is scheduled. */
  starts_at: string | null;
  opponent: { id: string; name: string };
  /** Times can still be proposed and accepted: the match hasn't started and has no room. */
  open: boolean;
  /** Proposed by the caller's team, for the opponent to accept. Earliest first. */
  sent: RescheduleProposal[];
  /** Proposed by the opponent, for the caller's team to accept. Earliest first. */
  received: RescheduleProposal[];
}

export interface MyReschedules {
  matches: RescheduleMatch[];
  /** The most times a team can have proposed for one match at once. */
  limit: number;
}

export interface MatchReferee {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  discord_id: string | null;
  discord_username: string | null;
}

/** One of a match's osu! multiplayer rooms. A room that breaks mid-match is replaced, and the old one stays listed. */
export interface MatchRoom {
  mp_id: number;
  url: string;
}

export interface Lobby<P extends Player = Player> {
  id: string;
  starts_at: string;
  /** Players can still join or leave: it hasn't started or been concluded. */
  open: boolean;
  referee: MatchReferee | null;
  /** Its osu! multiplayer rooms, oldest first; empty until the referee adds one. */
  rooms: MatchRoom[];
  concluded_at: string | null;
  /** In signup order. */
  teams: Team<P>[];
}

export interface StaffLobby extends Lobby<AdminTeamPlayer> {
  /** Staff-only. */
  referee_notes: string;
}

export interface PublicQualifiers {
  /** Earliest first. */
  lobbies: Lobby[];
  /** The most teams a lobby holds. */
  capacity: number;
  /** The qualifier stage, while its map pool is published; null otherwise. */
  pool_stage_id: string | null;
  /** The signed-in player's team, or null. */
  my_team_id: string | null;
}

export interface StaffQualifiers {
  lobbies: StaffLobby[];
  capacity: number;
  /** The qualifier stage, while its map pool is published; null otherwise. */
  pool_stage_id: string | null;
  /** Teams that haven't picked a lobby yet. */
  unassigned: Team<AdminTeamPlayer>[];
}

export type BracketSide = "winners" | "losers";

/** Where a match's winner or loser goes: a match and which of its two team slots. */
export interface NextMatch {
  match_id: string;
  slot: 1 | 2;
}

export interface MatchStreamer {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
}

export interface MatchStage {
  id: string;
  name: string;
  /** Whether its map pool is on the public mappool page. */
  published: boolean;
  /** Orders the stages. */
  position: number;
  /** How many maps the match is played to; null when not set. */
  best_of: number | null;
}

export interface BracketMatch {
  id: string;
  side: BracketSide;
  /** Numbered within its side: W1, W2, … and L1, L2, … */
  number: number;
  /** Top left corner on the editor's board, in board units (see lib/bracket.ts); can be negative. */
  pos_x: number;
  pos_y: number;
  /** null until the match is scheduled. */
  starts_at: string | null;
  team1_id: string | null;
  team2_id: string | null;
  team1_score: number | null;
  team2_score: number | null;
  /** The team that forfeited (1 or 2) when the match was decided that way; the other one won. */
  forfeit_slot: 1 | 2 | null;
  winner_to: NextMatch | null;
  loser_to: NextMatch | null;
  /** The stage the match is played in, which sets its map pool. */
  stage: MatchStage | null;
  referee: MatchReferee | null;
  streamer: MatchStreamer | null;
  /** Oldest first. */
  rooms: MatchRoom[];
  concluded_at: string | null;
}

export interface StaffBracketMatch extends BracketMatch {
  /** Staff-only. */
  referee_notes: string;
}

export interface PublicBracket {
  matches: BracketMatch[];
  /** Every team; matches refer to them by id. */
  teams: Team[];
  /** The signed-in player's team, or null. */
  my_team_id: string | null;
}

export interface StaffBracket {
  matches: StaffBracketMatch[];
  teams: Team<AdminTeamPlayer>[];
}

export type NotificationKind =
  | "match_rescheduled"
  | "confirm_time"
  | "staff_declined"
  | "reschedule_proposed"
  | "rank_over_limit"
  | "discord_left";
export type MatchStaffRole = "referee" | "streamer";

/**
 * For the staff, about a bracket match: match_rescheduled tells a host its teams moved it;
 * confirm_time asks its referee or streamer whether they can make the new time (after the teams
 * or a host moved it); staff_declined
 * tells everyone who could take their place that one of them couldn't.
 * For players: reschedule_proposed says the opponent proposed times for a match (one notice for
 * every proposal made before it's read); rank_over_limit says a rank update took the team over the
 * limit (also sent to the hosts); discord_left says a team member left the Discord server (to their
 * teammate and the hosts).
 */
export interface AppNotification {
  id: string;
  kind: NotificationKind;
  /** null for rank_over_limit. */
  match_id: string | null;
  side: BracketSide | null;
  number: number | null;
  team1_name: string | null;
  team2_name: string | null;
  /**
   * Who caused it: the player who accepted or proposed the time, the staff member who declined it,
   * or the player whose rank change took the team over the limit.
   */
  actor_username: string | null;
  /** confirm_time: a host moved the match in the bracket editor (the actor), not its teams. */
  by_host: boolean;
  /** null for match_rescheduled. */
  staff_role: MatchStaffRole | null;
  /** null when the match had no time before. */
  old_starts_at: string | null;
  /** null for the player notifications. */
  new_starts_at: string | null;
  /** The answer to a confirm_time. */
  response: "confirmed" | "declined" | null;
  responded_at: string | null;
  read: boolean;
  created_at: string;
  /** reschedule_proposed: the recipient's team; rank_over_limit: the team over the limit. */
  team_id: string | null;
  team_name: string | null;
  /** rank_over_limit: the actor's rank before and after, the team value it led to and the limit. */
  old_rank: number | null;
  new_rank: number | null;
  team_value: number | null;
  limit: number | null;
  /** reschedule_proposed: the opponent's proposals for the match that are still open. */
  pending_proposals: number;
  /** A confirm_time that can still be answered. */
  actionable: boolean;
}

export interface NotificationList {
  /** Newest first. */
  notifications: AppNotification[];
  /** Unread notifications plus confirmations still waiting for an answer. */
  attention: number;
}

export interface DevAccount {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  global_rank: number | null;
  roles: Role[];
  dummy: boolean;
  in_team: boolean;
  discord_linked: boolean;
}

export type MatchKind = "qualifier" | "bracket";

/** GET /api/matches/:id/room: what the referee page shows about a match. */
export interface RefereeRoom {
  id: string;
  kind: MatchKind;
  /** Bracket matches only (W1, L3). */
  side: BracketSide | null;
  number: number | null;
  /** Qualifiers only: the lobby's place by start time. */
  lobby_number: number | null;
  starts_at: string | null;
  referee: MatchReferee | null;
  /** Bracket matches only: who streams it, whom the referee can add as a referee of the room. */
  streamer: MatchStreamer | null;
  /** Oldest first; the last one is where the match is being played. */
  rooms: MatchRoom[];
  referee_notes: string;
  concluded_at: string | null;
  /** Bracket matches only: the teams playing blue (1, slots 1-2) and red (2, slots 3-4). */
  team1_id: string | null;
  team2_id: string | null;
  team1_score: number | null;
  team2_score: number | null;
  /** Bracket matches only: the team that forfeited (1 or 2), once decided that way. */
  forfeit_slot: 1 | 2 | null;
  /** A bracket match's teams, or a lobby's in signup order. */
  teams: Team<AdminTeamPlayer>[];
  /** The map pool it's played in, published or not. */
  stage: RoomStage | null;
  categories: Category[];
  slots: PublicSlot[];
  maps: PublicPoolMap[];
  /** Bracket matches only: the pool maps each team banned and picked, in the order they were made. */
  bans: MapBan[];
  picks: MapPick[];
  /** Bracket matches only: map wins the referee awarded by hand, oldest first. */
  manual_results: ManualResult[];
  /** Bracket matches only: the pool maps the saved score was counted from, in the order played. */
  map_results: MapResultRow[];
  /** The match has a Discord channel: clicking a player's Discord name pings them there. */
  discord_channel: boolean;
  /** What the room keeps about the match, the same for every referee and host on it. */
  settings: RoomSettings;
  /** The finished maps left out of the score ("Sayma"), by LiveFinished.key. */
  ignored_maps: string[];
  /** The room as its bot reads it, the same on every page; null for a match without a room. */
  live: LiveRoom | null;
}

/** The room options, last osu! sync and pick override for a match. */
export interface RoomSettings {
  /** !mp set's score mode (0 score, 1 accuracy, 2 combo, 3 Score V2). */
  score_mode: number;
  no_fail: boolean;
  auto_place: boolean;
  auto_kick: boolean;
  /** When the results were last synced from osu!'s record of the rooms, which replaces the chat's maps; null if never. */
  osu_synced_at: string | null;
  /** The team the referee said picks next, overriding the rules until the next ban or pick. */
  next_pick_team_id: string | null;
}

/** The stage a match is played in, which sets its map pool. */
export interface RoomStage {
  id: string;
  name: string;
  /** Whether its pool is on the public mappool page. */
  published: boolean;
  /** How many maps a bracket match in it is played to; null when not set. */
  best_of: number | null;
}

/**
 * GET /api/public/matches/:id: a match as its public page shows it. The referee page's view
 * without the referee's notes and the players' Discords; the pool is empty until its stage is
 * published.
 */
export interface PublicMatch {
  id: string;
  kind: MatchKind;
  side: BracketSide | null;
  number: number | null;
  lobby_number: number | null;
  starts_at: string | null;
  referee: MatchReferee | null;
  streamer: MatchStreamer | null;
  /** Oldest first; the last one is where the match is being played. */
  rooms: MatchRoom[];
  concluded_at: string | null;
  team1_id: string | null;
  team2_id: string | null;
  team1_score: number | null;
  team2_score: number | null;
  forfeit_slot: 1 | 2 | null;
  /** A bracket match's teams, or a lobby's in signup order. */
  teams: Team[];
  stage: RoomStage | null;
  categories: Category[];
  slots: PublicSlot[];
  maps: PublicPoolMap[];
  bans: MapBan[];
  picks: MapPick[];
  manual_results: ManualResult[];
  map_results: MapResultRow[];
  /** The team the referee said picks next, overriding the rules until the next ban or pick. */
  next_pick_team_id: string | null;
  live: LiveRoom | null;
}

/** Where a room's bot stands with it. */
export type RoomAccess =
  /** In its channel: it logs and runs the room. */
  | "in"
  /** Connected, on its way in. */
  | "joining"
  /** Bancho won't let it in: a room made elsewhere whose maker hasn't added it as a referee. */
  | "refused"
  /** Not connected to Bancho. */
  | "offline"
  /** No bot runs the room. */
  | "no_bot"
  | "closed";

export interface LiveRoomBadge {
  mp_id: number;
  /** The bot that runs it, by osu! username. */
  bot: string | null;
  access: RoomAccess;
}

export interface LiveSlot {
  /** 1-16. */
  slot: number;
  username: string;
  /** Known from !mp settings; joins only give the name. */
  user_id: number | null;
  team: "red" | "blue" | null;
  status: "ready" | "not_ready" | "no_map";
  host: boolean;
  /** Mods the player picked under freemod, as acronyms ("HDHR"), from !mp settings. */
  mods: string;
  mods_known: boolean;
}

export interface LiveScore {
  username: string;
  /** Known when it's from osu!'s record or the player was read from !mp settings. */
  user_id: number | null;
  score: number;
  passed: boolean;
  /** The mods the player played with, the room's included; null when unknown. */
  mods: string | null;
}

/** A room as BanchoBot has described it. */
export interface LiveRoomState {
  name: string | null;
  beatmap_id: number | null;
  beatmap_name: string | null;
  team_mode: string | null;
  win_condition: string | null;
  /** As BanchoBot last described them: "HardRock, NoFail", "Freemod" or "". */
  mods: string | null;
  size: number | null;
  locked: boolean | null;
  playing: boolean;
  closed: boolean;
  slots: LiveSlot[];
  /** Scores of the map being played. */
  current: LiveScore[];
  /** The room's referees (!mp addref), lowercase with spaces. */
  refs: string[];
  /** When the map being played started. */
  map_started_at: string | null;
}

/** A finished map of one of the match's rooms. */
export interface LiveFinished {
  /** "<source>:<room>:<n>"; "Sayma" is keyed by it. */
  key: string;
  mp_id: number;
  beatmap_id: number | null;
  beatmap_name: string | null;
  /** The room's mods as acronyms, without freemod. */
  room_mods: string;
  freemod: boolean;
  scores: LiveScore[];
  /** The pool map it was (PublicPoolMap.id); null outside the pool (a warmup, a mistake). */
  map_id: string | null;
  /** Each team's summed score on it, best first. */
  totals: { team_id: string; score: number }[];
  /** Bracket matches: the team that won it; null for a tie. */
  winner_id: string | null;
  /** It counts towards the score: not left out with "Sayma". */
  counted: boolean;
}

/**
 * The match's room as its bot reads it (GET .../room, GET /api/public/matches/:id and the live
 * socket's `room`): the one reading every page shows.
 */
export interface LiveRoom {
  /** Every room of the match, oldest first, with its bot. */
  rooms: LiveRoomBadge[];
  /** The latest room, where the match is played. */
  state: LiveRoomState;
  /** The pool map the room has on. */
  current_map_id: string | null;
  /** Every room's finished maps, oldest room first. */
  finished: LiveFinished[];
  /** Bracket matches with both teams: the map wins counted from the rooms. */
  tally: { team1: number; team2: number } | null;
  /** The site has that score and the maps behind it. */
  score_saved: boolean;
  /** Whose pick is next, while there's a pick to make. */
  next_pick_team_id: string | null;
  /** The results come from osu!'s record of the rooms, up to its last map. */
  osu_synced: boolean;
}

/** A command for the room's bot (POST /api/matches/:id/room/command). */
export type RoomCommand =
  | { type: "set_up" }
  | { type: "settings" }
  | { type: "invite"; osu_id: number }
  | { type: "invite_all" }
  | { type: "move"; slot: number; to: number | null; team: "red" | "blue" | null }
  | { type: "place_all" }
  | { type: "kick"; slot: number }
  | { type: "kick_outsiders" }
  | { type: "lock"; locked: boolean }
  | { type: "timer"; seconds: number }
  | { type: "abort_timer" }
  | { type: "start"; seconds: number }
  | { type: "abort" }
  | { type: "close" }
  | { type: "map"; map_id: string }
  | { type: "announce_score" }
  | { type: "add_ref"; username: string }
  | { type: "remove_ref"; username: string }
  | { type: "rejoin" }
  /** Hosts only: says something in the room through its bot. */
  | { type: "say"; text: string };

/** How a bot account is doing. */
export type BotState = "online" | "connecting" | "off" | "login_failed" | "full";

export interface BotRoomView {
  mp_id: number;
  match_id: string;
  /** "W3", "Lobi 2"; null for a match that's over. */
  label: string | null;
  /** The bot made it, rather than being added to it. */
  made: boolean;
  access: RoomAccess;
}

/** GET /api/bancho/bots: a bot account (never its password) and how it's doing. */
export interface BanchoBot {
  id: string;
  username: string;
  active: boolean;
  created_at: string;
  status: {
    state: BotState;
    /** Why it's offline, when not by choice. */
    error: string | null;
    online_since: string | null;
    /** The rooms that count against its limit. */
    rooms: BotRoomView[];
    queued: number;
    recent: number;
  } | null;
}

export interface BotsView {
  bots: BanchoBot[];
  limits: { rooms: number; lines: number; window_secs: number };
}

/** Someone who has a match's referee page open (sent live to staff). */
export interface RefereeViewer {
  id: string;
  username: string;
  avatar_url: string;
}

/** A finished pool map that counts towards a bracket match's score, with each team's summed score. */
export interface MapResultRow {
  /** PublicPoolMap.id; null once the map was deleted from the site. */
  map_id: string | null;
  team1_score: number;
  team2_score: number;
  /** Each player's score on it, best first. */
  players: MapPlayerScore[];
}

export interface MapPlayerScore {
  /** The player's user id (AdminTeamPlayer.id). */
  user_id: string;
  score: number;
  /** Acronyms, the room's included ("HDNF"); null when they weren't read. */
  mods: string | null;
  passed: boolean;
}

/** A map win the referee awarded by hand: counts as one map won by the team. */
export interface ManualResult {
  id: string;
  /** A pool map (PublicPoolMap.id), or null for a point given without one. */
  map_id: string | null;
  team_id: string;
  note: string;
}

/** A pool map (PublicPoolMap.id) banned in a match by one of its teams. */
export interface MapBan {
  map_id: string;
  team_id: string;
}

/** A pool map (PublicPoolMap.id) picked in a match by one of its teams. */
export interface MapPick {
  map_id: string;
  team_id: string;
}

export interface RoomChatMessage {
  id: number;
  sent_at: string;
  sender: string;
  text: string;
  /** A /me line. */
  action: boolean;
}

/**
 * One room's saved chat, oldest first. GET /api/matches/:id/chat (and the public
 * /api/public/matches/:id/chat) return one per room.
 */
export interface RoomChat {
  mp_id: number;
  messages: RoomChatMessage[];
}

/** What hosts pick on /admin/discord. Discord ids are strings; null means unset. */
export interface DiscordSettings {
  verified_role_id: string | null;
  player_role_id: string | null;
  team_announce_channel_id: string | null;
  match_announce_channel_id: string | null;
  results_channel_id: string | null;
  host_channel_id: string | null;
  referee_channel_id: string | null;
  streamer_channel_id: string | null;
  qualifier_category_id: string | null;
  qualifier_archive_category_id: string | null;
  bracket_category_id: string | null;
  bracket_archive_category_id: string | null;
}

export interface GuildRole {
  id: string;
  name: string;
  /** 0 for no colour. */
  color: number;
}

export interface GuildChannel {
  id: string;
  name: string;
  /** The category a text channel is in. */
  parent_id: string | null;
}

/** GET /api/discord: the bot's state, its settings and what the server has to pick from. */
export interface DiscordView {
  /** DISCORD_BOT_TOKEN and DISCORD_GUILD_ID are set. */
  enabled: boolean;
  /** Hosts have it switched on (PUT /api/discord/active); while off it's disconnected and queues nothing. */
  active: boolean;
  connected: boolean;
  /** null when the bot isn't set up or Discord couldn't be reached. */
  guild: { roles: GuildRole[]; text_channels: GuildChannel[]; categories: GuildChannel[] } | null;
  settings: DiscordSettings;
}

/** The bracket matches the signed-in user is pinged for on Discord. */
export interface Subscriptions {
  match_ids: string[];
  /** The Discord bot is set up and switched on; without it there's nothing to subscribe to. */
  enabled: boolean;
  /** Subscribing needs a linked Discord account. */
  discord_linked: boolean;
}
