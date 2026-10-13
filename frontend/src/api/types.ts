// Response shapes returned by the Rust backend (see backend/src/db).

export type Role = "host" | "map_pooler" | "basic";

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
