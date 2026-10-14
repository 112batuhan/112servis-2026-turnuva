// Response shapes returned by the Rust backend (see backend/src/db).

export type Role = "host" | "map_pooler" | "referee" | "streamer" | "basic";

export interface DiscordAccount {
  id: string;
  osu_user_id: string;
  discord_id: string;
  username: string;
  avatar: string | null;
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
  role: Role;
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
  role: Role;
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

export interface DevAccount {
  id: string;
  osu_id: number;
  username: string;
  avatar_url: string;
  global_rank: number | null;
  role: Role;
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
  /** Oldest first; the last one is where the match is being played. */
  rooms: MatchRoom[];
  referee_notes: string;
  concluded_at: string | null;
  /** Bracket matches only: the teams playing red (1) and blue (2). */
  team1_id: string | null;
  team2_id: string | null;
  team1_score: number | null;
  team2_score: number | null;
  /** A bracket match's teams, or a lobby's in signup order. */
  teams: Team<AdminTeamPlayer>[];
  /** The map pool it's played in, published or not. */
  stage: { id: string; name: string } | null;
  categories: Category[];
  slots: PublicSlot[];
  maps: PublicPoolMap[];
  /** Bracket matches only: the pool maps each team banned and picked, in the order they were made. */
  bans: MapBan[];
  picks: MapPick[];
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

/** One room's saved chat, oldest first. GET /api/matches/:id/chat returns one per room. */
export interface RoomChat {
  mp_id: number;
  messages: RoomChatMessage[];
}
