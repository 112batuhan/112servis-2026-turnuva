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
