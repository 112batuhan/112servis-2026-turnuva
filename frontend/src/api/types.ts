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
  /** Set when the hosts banned them from taking part; they can still sign in and look. */
  ban: UserBan | null;
}

export interface UserBan {
  /** Empty when the host gave none. */
  reason: string;
  created_at: string;
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
  /** Set when banned: why (empty when no reason was given), when and by whom. */
  ban_reason: string | null;
  banned_at: string | null;
  banned_by_username: string | null;
}

export interface Stage {
  id: string;
  name: string;
  /** Its short name for the bracket board's cards ("QF"); null: none. */
  abbreviation: string | null;
  position: number;
  published: boolean;
  /** The qualifier lobbies' map pool; at most one stage is. */
  is_qualifier: boolean;
  /** How many maps its bracket matches are played to (odd); null until the hosts set it. */
  best_of: number | null;
  /** The order its bracket matches ban and pick in ("p1-p2-b2-b1-p1-*", lib/draftOrder.ts); null: the default. */
  draft_order: string | null;
  /** Where its beatmap pack is downloaded from (hosted elsewhere); null: none yet. */
  pack_url: string | null;
  created_at: string;
  updated_at: string;
}

/** Where a stage is on the tournament's track (backend `stage_track.rs`, the only place that decides it). */
export type StageState = "done" | "current" | "upcoming";

/**
 * A stage as `/api/public/stages` has it: every stage, published or not (the track names every
 * round), without its draft order or pack link. Only published ones have a pool or statistics to open.
 */
export interface PublicStage {
  id: string;
  name: string;
  position: number;
  published: boolean;
  is_qualifier: boolean;
  best_of: number | null;
  /** Its matches with a saved map (the qualifier's: its lobbies). */
  played: number;
  /** Its matches being played now: a room open, not concluded. */
  live: number;
  state: StageState;
}

/** What a pool category is: its maps' rules (backend/src/category.rs, lib/category.ts). */
export type CategoryKind = "nomod" | "modded" | "freemod" | "tiebreaker";

export interface Category {
  id: string;
  stage_id: string;
  name: string;
  /** Its slots' labels: "NM" makes NM1, NM2…; what players type to ban and pick. */
  abbreviation: string;
  kind: CategoryKind;
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
  /** The map under HR, for freemod picks: only on nomod maps (null on maps added with mods,
   *  and on ones added before HR stats were stored until they're recomputed). BPM and length
   *  don't change. */
  hr_star_rating: number | null;
  hr_cs: number | null;
  hr_ar: number | null;
  hr_od: number | null;
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

export type Eligibility =
  "eligible" | "not_player" | "banned" | "in_team" | "unranked" | "over_limit";

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

/** A team's place in the qualifier, which seeds the bracket (backend `services::stats::QualifierPlace`). */
export interface QualifierPlace {
  rank: number;
  /** Its lobby isn't over, so the place can still change. */
  pending: boolean;
  /** A bracket was made and it isn't in it: the seeding's cut left it out of the tournament. */
  eliminated: boolean;
}

/** A team in the team lists, with its qualifier place (null until it has one; public only once the qualifier's pool is). */
export type ListedTeam<P extends Player = Player> = Team<P> & { qualifier: QualifierPlace | null };

export interface TeamList<P extends Player = Player> {
  teams: ListedTeam<P>[];
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
  /** Banned by the hosts; the team stays until a host disbands it. */
  banned: boolean;
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
  /** A pairing its bracket match (side, number) may still turn out to be: played only if both teams get there. */
  potential: boolean;
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
  /** Its letter, fixed when it's created: "A", "B". */
  name: string;
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
  /** Its short name, on the bracket board's cards ("QF"); null: none. */
  abbreviation: string | null;
  /** Whether its map pool is on the public mappool page. */
  published: boolean;
  /** Orders the stages. */
  position: number;
  /** How many maps the match is played to; null when not set. */
  best_of: number | null;
}

/**
 * A pairing an undecided bracket match may turn out to be: when a slot waits on another match of the same
 * stage whose teams are both known, every combination of the teams that can reach it gets one. Its teams
 * agree on its time and its referee and streamer sign up, but it has no room, score or page of its own.
 * Once the match is decided, the matching one hands it its time and staff and the rest go.
 */
export interface PotentialMatch {
  id: string;
  team1_id: string;
  team2_id: string;
  starts_at: string | null;
  referee: MatchReferee | null;
  streamer: MatchStreamer | null;
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
  /** The pairings it may still turn out to be, soonest first; empty once it's decided. */
  potentials: PotentialMatch[];
  /**
   * Only on the stand-ins `potentialRows` (lib/bracket.ts) makes for the list, the calendar and the
   * fixture rows: the bracket match this potential match is a pairing of. Its id is the potential match's.
   */
  potential_of?: string;
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
  | "discord_left"
  | "reschedule_accepted"
  | "banned"
  | "teammate_banned"
  | "player_banned"
  | "player_report"
  | "emergency_stop"
  | "autoref_stopped"
  | "no_show_forfeit"
  | "lobby_no_show";
export type MatchStaffRole = "referee" | "streamer";

/**
 * For the staff, about a bracket match: match_rescheduled tells a host its teams moved it;
 * confirm_time asks its referee or streamer whether they can make the new time (after the teams
 * or a host moved it); staff_declined
 * tells everyone who could take their place that one of them couldn't.
 * For players: reschedule_proposed says the opponent proposed times for a match (one notice for
 * every proposal made before it's read); reschedule_accepted says a player accepted a proposed time
 * and the match moved (to every player of both teams but them); rank_over_limit says a rank update took the team over the
 * limit (also sent to the hosts); discord_left says a team member left the Discord server (to their
 * teammate and the hosts); banned says the hosts banned the recipient (with the reason; about no
 * match or team); teammate_banned says they banned the recipient's teammate (the actor; no reason).
 * For the hosts but the one who did it, player_banned says a player (the actor) was banned, with the
 * reason and their team if they're in one.
 * From the auto referee, to every host and referee: player_report is a player's -rapor (the actor,
 * their team, what they wrote), emergency_stop a player's -durdur (the match went to manual), autoref_stopped that it handed the
 * match over to the staff (why, in `message`). To players: no_show_forfeit says a bracket match was
 * forfeited for the team that didn't show up (`team_id`; to both teams), lobby_no_show that their
 * team was taken out of its qualifier lobby for not showing up.
 */
export interface AppNotification {
  id: string;
  kind: NotificationKind;
  /** null for rank_over_limit. */
  match_id: string | null;
  /** The match is a pairing its bracket match (side, number) may still turn out to be. */
  potential: boolean;
  /** The bracket match to open for it: the match itself, or the one a potential match is a pairing of. */
  bracket_match_id: string | null;
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
  /** null for the player notifications but reschedule_accepted. */
  new_starts_at: string | null;
  /** The answer to a confirm_time. */
  response: "confirmed" | "declined" | null;
  responded_at: string | null;
  read: boolean;
  created_at: string;
  /** reschedule_proposed, reschedule_accepted: the recipient's team; rank_over_limit: the team over the limit. */
  team_id: string | null;
  team_name: string | null;
  /** rank_over_limit: the actor's rank before and after, the team value it led to and the limit. */
  old_rank: number | null;
  new_rank: number | null;
  team_value: number | null;
  limit: number | null;
  /** banned, player_banned: the reason the host gave (empty when none); null for every other kind. */
  ban_reason: string | null;
  /** player_report: what the player wrote; autoref_stopped: why the auto referee stopped. */
  message: string | null;
  /** The qualifier lobby it's about, "B". */
  lobby_name: string | null;
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
  team_name: string | null;
  discord_linked: boolean;
}

/** What the dev switcher's auto referee test offers (`GET /api/dev/autoref-test`). */
export interface DevAutorefTest {
  /** Bracket stages with a best of, a pick and ban order and a pool. */
  stages: { id: string; name: string }[];
  /** Off, the test's room isn't opened by itself. */
  auto_rooms: boolean;
  /** The test's own teams, blue then red: their players can be picked again. */
  test_teams: string[];
}

export type MatchKind = "qualifier" | "bracket";

/** GET /api/matches/:id/room: what the referee page shows about a match. */
export interface RefereeRoom {
  id: string;
  kind: MatchKind;
  /** Bracket matches only (W1, L3). */
  side: BracketSide | null;
  number: number | null;
  /** Qualifiers only: the lobby's letter, "A". */
  lobby_name: string | null;
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
  /** Bracket matches only: the bans and picks that ran out on the auto referee's clock, in order. */
  missed: MissedTurn[];
  /** Bracket matches only: map wins the referee awarded by hand, oldest first. */
  manual_results: ManualResult[];
  /** Bracket matches only: the pool maps the saved score was counted from, in the order played. */
  map_results: MapResultRow[];
  /** The match has a Discord channel: clicking a player's Discord name pings them there. */
  discord_channel: boolean;
  /** What the room keeps about the match, the same for every referee and host on it. */
  settings: RoomSettings;
  /** The hosts have the auto referee open rooms by itself (Bancho bots page). */
  auto_rooms: boolean;
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
  /** The team the referee said goes next, overriding the order until the next ban or pick. */
  next_pick_team_id: string | null;
  /** The room's bot referees the match by itself (backend/src/bancho/autoref.rs); false: manual. */
  auto_ref: boolean;
}

/** The stage a match is played in, which sets its map pool. */
export interface RoomStage {
  id: string;
  name: string;
  /** Whether its pool is on the public mappool page. */
  published: boolean;
  /** How many maps a bracket match in it is played to; null when not set. */
  best_of: number | null;
  /** The order its bracket matches ban and pick in, the default when the hosts set none (lib/draftOrder.ts). */
  draft_order: string;
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
  lobby_name: string | null;
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
  missed: MissedTurn[];
  manual_results: ManualResult[];
  map_results: MapResultRow[];
  /** The team the referee said goes next, overriding the order until the next ban or pick. */
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
  /** The results come from osu!'s record of the rooms, up to its last map. */
  osu_synced: boolean;
  /** Where the auto referee stands; null when the bots don't run the match. */
  autoref: AutorefView | null;
}

/**
 * The auto referee's phase: before the match (waiting), a bracket match's warmups, roll and
 * bans/picks (draft), a map on and waiting for ready (ready), being played, its scores read from
 * osu! (scoring), between maps (next), over (done).
 */
export type AutorefPhase =
  "waiting" | "warmup" | "roll" | "draft" | "ready" | "playing" | "scoring" | "next" | "done";

export interface AutorefView {
  /** The match is in auto mode. */
  enabled: boolean;
  phase: AutorefPhase;
  /** Warmup: the team with the host; draft: the team to ban or pick. */
  team_id: string | null;
  step: "ban" | "pick" | null;
  /** Ready, playing, scoring: the pool map on. */
  map_id: string | null;
  /** When the phase's timer runs out (waiting: the match's time, then the no-show cutoff). */
  deadline: string | null;
  /** Each team's -abort uses left. */
  teams: { team_id: string; aborts_left: number }[];
}

/** A line of the auto referee's log. */
export interface AutorefLogEntry {
  id: number;
  level: "info" | "warn" | "error";
  message: string;
  created_at: string;
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
  /** Says something in the room through its bot, signed with the sender's name. */
  | { type: "say"; text: string };

/** How a bot account is doing. */
export type BotState = "online" | "connecting" | "off" | "login_failed" | "full";

export interface BotRoomView {
  mp_id: number;
  match_id: string;
  /** "W3", "Lobi B"; null for a match that's over. */
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
  /** The auto referee opens match rooms by itself, 10 minutes before each match (app_settings). */
  auto_rooms: boolean;
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
  /** When the referee marked it: bans and picks together give the draft's order. */
  created_at: string;
}

/** A pool map (PublicPoolMap.id) picked in a match by one of its teams. */
export interface MapPick {
  map_id: string;
  team_id: string;
  /** When the referee marked it. */
  created_at: string;
}

/**
 * A ban or pick a team let run out on the auto referee's clock: a missed ban takes its step of the
 * order with no map banned, a missed pick passes the step to the other team (lib/draftOrder.ts).
 */
export interface MissedTurn {
  id: string;
  team_id: string;
  kind: "ban" | "pick";
  created_at: string;
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
  /** Hosts stopped new match channels from opening; the ones there are still kept up to date. */
  channels_paused: boolean;
  /** How many matches have a Discord channel. */
  channel_count: number;
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

/** POST /api/kupa: `reply` is the reply of the message this guess entered. */
export interface ArgGuessResult {
  correct: boolean;
  reply: string | null;
}

// ---- pick'em (handlers/pickem.rs) ----

/** How a stage's pick'em opens and locks: by the rule, or as a host set it. */
export type PickemMode = "auto" | "open" | "closed";

/**
 * upcoming: waiting for the stage before it to be played; open: taking predictions; locked: no more
 * predictions, the split is public and results come in; finished: locked with every match over.
 */
export type PickemStatus = "upcoming" | "open" | "locked" | "finished";

/** Whether the viewer can predict, and if not, what's missing. */
export type CanPredict = "yes" | "sign_in" | "discord" | "banned";

export interface PickemStage {
  id: string;
  name: string;
  position: number;
  best_of: number | null;
  mode: PickemMode;
  status: PickemStatus;
  /** When the stage before it was played out; null while it isn't, or without one. */
  opens_at: string | null;
  /** When predictions next lock: its first match's time, or with `mode` "open" the next match's still to start. */
  closes_at: string | null;
  /** The stage it waits for. */
  previous: string | null;
  /** How many people have predicted at least one of its matches. */
  pickers: number;
}

export interface MyPrediction {
  match_id: string;
  team_id: string;
  /** The winner's maps, then the loser's. */
  winner_score: number;
  loser_score: number;
  /** What it scored, once the match has a result. */
  points: number | null;
  /** The team it's on has been taken out of the match, so it can't score. */
  void: boolean;
}

export interface PickemScoreCount {
  /** The team picked to win. */
  slot: 1 | 2;
  winner_score: number;
  loser_score: number;
  count: number;
}

/** How a locked match was predicted. */
export interface PickemSplit {
  match_id: string;
  total: number;
  team1: number;
  team2: number;
  scores: PickemScoreCount[];
}

export interface PickemTally {
  points: number;
  /** Predictions that got both the winner and the score. */
  exact: number;
  /** Predictions that got the winner. */
  winners: number;
  /** Predictions on matches that have a result. */
  scored: number;
  /** Every prediction made. */
  picks: number;
}

export interface PickemStanding extends PickemTally {
  /** Shared by everyone level on points and exact predictions. */
  rank: number;
  player: Player;
  stages: (PickemTally & { stage_id: string })[];
}

/** GET /api/public/pickem. Matches and teams come from the public bracket. */
export interface PickemOverview {
  stages: PickemStage[];
  can_predict: CanPredict;
  mine: MyPrediction[];
  /** Every match of a locked stage with both teams. */
  splits: PickemSplit[];
  standings: PickemStanding[];
}

/** GET /api/public/pickem/matches/:id; null for a match outside every stage's pick'em. */
export interface MatchPickem {
  stage_id: string;
  stage_name: string;
  /** The stage's, except that a match that has started in a stage a host opened is locked on its own. */
  status: PickemStatus;
  /** When predictions on this match lock. */
  closes_at: string | null;
  /** Whether it can be predicted now (the viewer's account aside). */
  predictable: boolean;
  can_predict: CanPredict;
  /** How many have predicted it so far. */
  total: number;
  /** How they predicted it, once it's locked. */
  split: PickemSplit | null;
  mine: MyPrediction | null;
}

/** GET /api/public/stats: how far each published stage has got. */
export interface StageActivity {
  stage_id: string;
  /** Matches with a saved map (the qualifier's: its lobbies). */
  played: number;
  /** Matches being played now (a room, not concluded). */
  live: number;
}

/** The plays on a map: how many, their mean and their sample standard deviation. */
export interface StatField {
  count: number;
  mean: number;
  sd: number;
}

export interface StatPlayerPlay {
  user_id: string;
  score: number;
  /** 0..1, from osu!'s record of the room; null until it's been read. */
  accuracy: number | null;
  mods: string | null;
  passed: boolean;
  /** The Z-percentile among every player's play on the map: the share of a normal field it beats,
   * 0..1. */
  points: number;
  /** Among every player's play on the map; level scores share it. */
  rank: number;
}

/** A team's play on a map: its players' scores summed. */
export interface StatTeamPlay {
  match_id: string;
  map_id: string;
  team_id: string;
  score: number;
  accuracy: number | null;
  /** The Z-percentile among the teams' plays on the map, 0..1. */
  points: number;
  rank: number;
  /** Bracket matches: whether the team won the map; null for a tie (or a lobby). */
  won: boolean | null;
  picked: boolean;
  /** Best first. */
  players: StatPlayerPlay[];
}

export interface StatMap {
  map_id: string;
  teams: StatField;
  players: StatField;
  avg_accuracy: number | null;
  picks: number;
  bans: number;
  picker_wins: number;
  picker_plays: number;
  best_team: { team_id: string; match_id: string; score: number; accuracy: number | null } | null;
  best_player: {
    user_id: string;
    team_id: string;
    match_id: string;
    score: number;
    accuracy: number | null;
    mods: string | null;
  } | null;
}

/** A team's or a player's standing on one map. */
export interface StatCell {
  map_id: string;
  /** The play's points, its Z-percentile (a bracket stage's plays averaged); -1 for a missing
   * qualifier map. */
  points: number;
  score: number | null;
  accuracy: number | null;
  rank: number | null;
  plays: number;
  missing: boolean;
}

/** A team's bans and picks of one map over a bracket stage: its own, and its opponents' against it. */
export interface StatDraftTally {
  map_id: string;
  picked: number;
  banned: number;
  picked_against: number;
  banned_against: number;
}

export interface StatStanding {
  /** The team's, or the player's. */
  id: string;
  team_id: string;
  /** The seed in the qualifier; null until there's anything to rank. */
  rank: number | null;
  /** The sum of the maps' points, missing ones included: what the qualifier seeds by. */
  points: number;
  /** The plays' average points: what a bracket stage ranks by. */
  points_avg: number;
  plays: number;
  /** The maps counted in points, missing ones included. */
  maps_counted: number;
  total_score: number;
  avg_score: number;
  avg_accuracy: number | null;
  /** Players: the plays on which they outscored their teammate. */
  team_best: number;
  /** Qualifier: their lobby isn't over, so missing maps don't count yet. */
  pending: boolean;
  best: {
    map_id: string;
    match_id: string;
    points: number;
    score: number;
    rank: number;
    field: number;
  } | null;
  per_map: StatCell[];
  /** The maps the team (a player's team) or its opponents picked or banned, in pool order. */
  drafts: StatDraftTally[];
}

export interface StatMatch {
  id: string;
  /** "W3", "L7", "Lobi B". */
  code: string;
  side: BracketSide | null;
  concluded: boolean;
  live: boolean;
}

/** GET /api/public/stats/:id — a published stage's statistics, from the scores the site saved. */
export interface StageStats {
  pool: PublicStageDetail;
  matches: StatMatch[];
  teams: Team[];
  stats: {
    matches_played: number;
    plays: StatTeamPlay[];
    maps: StatMap[];
    teams: StatStanding[];
    players: StatStanding[];
  };
}

// ---------- seeding (/admin/seeding, backend services/seeding.rs) ----------

/** The bracket a seeding makes: double elimination from a Round of 32 or a Round of 16. */
export type BracketFormat = "round_of_32" | "round_of_16";

/** What the qualifier says of a team. */
export interface SeedStanding {
  rank: number;
  /** The sum of its points (Z-percentiles) over the pool: what the qualifier ranks by. */
  points: number;
  total_score: number;
  /** Maps played, and maps counted as -1 (no score though the lobby is over). */
  plays: number;
  missing: number;
  /** Its lobby isn't over, so its missing maps don't count yet. */
  pending: boolean;
}

export interface SeedTeam extends Team<AdminTeamPlayer> {
  /** null: no place in the qualifier (it played no lobby). */
  qualifier: SeedStanding | null;
  /** The qualifier lobby it signed up for ("B"). */
  lobby: string | null;
  /** The hosts banned one of its players; the qualifier's order puts it last. */
  banned: boolean;
  /** It plays in the bracket. */
  in_bracket: boolean;
  /** Its seed in the qualifier's order (banned teams last), from 1: where it is unless a host moved it. */
  qualifier_seed: number;
}

export interface Seeding {
  /** In seeding order, best seed first. */
  teams: SeedTeam[];
  /** A host rearranged it: it's no longer the qualifier's order. */
  custom: boolean;
  qualifier_stage_id: string | null;
  pool_size: number;
  /** Qualifier lobbies with teams that aren't concluded; a bracket waits for them. */
  lobbies_not_over: number;
  /** The bracket's matches, and how many have been played (concluded or with a room). */
  matches: number;
  played: number;
  /** The bracket stages in play order. */
  stages: Stage[];
}
