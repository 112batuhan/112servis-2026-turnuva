// A multiplayer room as the pages show it. The site's bots read every room from BanchoBot's
// lines and send its !mp commands (backend/src/bancho); the pages get the result (LiveRoom) and
// only shape it for display here.

import type { LiveRoomState, LiveScore } from "../api/types";

export const BANCHO_BOT = "BanchoBot";

export type SlotTeam = "red" | "blue";
export type SlotStatus = "ready" | "not_ready" | "no_map";

export interface RoomSlot {
  /** 1-16. */
  slot: number;
  username: string;
  /** Known from !mp settings; joins only give the name. */
  userId: number | null;
  team: SlotTeam | null;
  status: SlotStatus;
  host: boolean;
  /** Mods the player picked, with freemod on, as acronyms ("HDHR"); from !mp settings. */
  mods: string;
  /** Whether `mods` came from !mp settings; a player who just joined hasn't been read yet. */
  modsKnown: boolean;
}

export interface MapScore {
  username: string;
  /** Known when the bot read the player from !mp settings, or the score came from osu!. */
  userId: number | null;
  score: number;
  passed: boolean;
  /** The mods the player played with, the room's included; null when unknown. */
  mods: string | null;
}

export interface MapResult {
  beatmapId: number | null;
  beatmapName: string | null;
  /** The room's own mods as acronyms ("HRNF"), without freemod. */
  roomMods: string;
  /** Whether the players picked their own mods. */
  freemod: boolean;
  scores: MapScore[];
}

export interface RoomState {
  name: string | null;
  beatmapId: number | null;
  beatmapName: string | null;
  teamMode: string | null;
  winCondition: string | null;
  /** As BanchoBot last described them: "HardRock, NoFail", "Freemod" or "". */
  mods: string | null;
  size: number | null;
  locked: boolean | null;
  playing: boolean;
  closed: boolean;
  slots: RoomSlot[];
  /** Scores of the map being played. */
  current: MapScore[];
  /** Who was added as a referee of the room (!mp addref), by normalizeName. */
  refs: string[];
  /** When the map being played started, ms. */
  mapStartedAt: number | null;
}

export const EMPTY_ROOM: RoomState = {
  name: null,
  beatmapId: null,
  beatmapName: null,
  teamMode: null,
  winCondition: null,
  mods: null,
  size: null,
  locked: null,
  playing: false,
  closed: false,
  slots: [],
  current: [],
  refs: [],
  mapStartedAt: null,
};

export function fromLiveScore(s: LiveScore): MapScore {
  return { username: s.username, userId: s.user_id, score: s.score, passed: s.passed, mods: s.mods };
}

/** The server's reading of a room, as the pages use it. */
export function fromLiveState(s: LiveRoomState): RoomState {
  return {
    name: s.name,
    beatmapId: s.beatmap_id,
    beatmapName: s.beatmap_name,
    teamMode: s.team_mode,
    winCondition: s.win_condition,
    mods: s.mods,
    size: s.size,
    locked: s.locked,
    playing: s.playing,
    closed: s.closed,
    slots: s.slots.map((slot) => ({
      slot: slot.slot,
      username: slot.username,
      userId: slot.user_id,
      team: slot.team,
      status: slot.status,
      host: slot.host,
      mods: slot.mods,
      modsKnown: slot.mods_known,
    })),
    current: s.current.map(fromLiveScore),
    refs: s.refs,
    mapStartedAt: s.map_started_at ? Date.parse(s.map_started_at) : null,
  };
}

/** Usernames compare without case, and IRC writes spaces as underscores. */
export function sameName(a: string, b: string): boolean {
  return normalizeName(a) === normalizeName(b);
}

export function normalizeName(name: string): string {
  return name.trim().replace(/_/g, " ").toLowerCase();
}

/** The name to log in or address someone with on IRC. */
export function ircName(username: string): string {
  return username.trim().replace(/ /g, "_");
}

/** "HDHR" -> ["HD", "HR"]; "NM" and "" are no mods. */
export function splitMods(acronyms: string): string[] {
  return (acronyms.toUpperCase().match(/[A-Z0-9]{2}/g) ?? []).filter((m) => m !== "NM");
}

/** Whether the room's mods, as BanchoBot described them, include freemod. */
export function isFreemod(mods: string | null): boolean {
  return /free ?mod/i.test(mods ?? "");
}

/** A pool category is freemod when it's named FM or TB (FM1, TB, …). */
export function isFreemodCategory(name: string): boolean {
  return /^(FM|TB)/i.test(name.trim());
}

/** A freemod category whose maps show their HR stats too: FM, where players may take HR. The
 *  tiebreaker is freemod as well but doesn't need them. */
export function showsHrStats(categoryName: string): boolean {
  return /^FM/i.test(categoryName.trim());
}

/** Whether someone was added as a referee of the room. */
export const isRoomRef = (room: Pick<RoomState, "refs">, username: string) => room.refs.includes(normalizeName(username));

// ---------- the chat as spectators read it ----------

const OSU = String.raw`https://osu\.ppy\.sh`;

// BanchoBot's lines a spectator follows the match by: maps set, started, finished, aborted, the
// room closing, and the roll for who picks first. Everything else it says (joins, slot moves,
// settings dumps, timers, locks, mods, and each player's "finished playing", which the score
// sheet already lists) is the room being run.
const MATCH_LINES = [
  /^The match has (started|finished)!$/,
  /^Aborted the match$/,
  /^Closed the match$/,
  new RegExp(String.raw`^Changed beatmap to ${OSU}/b/(\d+) (.*)$`),
  new RegExp(String.raw`^Beatmap changed to: (.*) \(${OSU}/b/(\d+)\)$`),
  /^.+ rolls \d+ point\(s\)$/,
];

/**
 * Whether a room line is the room being run rather than the match being played: BanchoBot's
 * housekeeping, and !mp commands. The public match page hides these unless asked.
 */
export function isRoomNoise(from: string, text: string): boolean {
  if (from === BANCHO_BOT) return !MATCH_LINES.some((re) => re.test(text));
  return /^!mp\b/i.test(text.trim());
}

// ---------- the room's setup ----------

export type ScoreMode = 0 | 1 | 2 | 3;
export const SCORE_MODES: { value: ScoreMode; label: string }[] = [
  { value: 3, label: "Score V2" },
  { value: 0, label: "Score" },
  { value: 1, label: "Accuracy" },
  { value: 2, label: "Combo" },
];

/** The colour each side of a bracket match plays as in the room: team 1 blue, team 2 red. */
export const SIDE_COLOR: Record<1 | 2, SlotTeam> = { 1: "blue", 2: "red" };

/** The slots each side's two players sit in: team 1 (blue) in 1 and 2, team 2 (red) in 3 and 4. */
export const SIDE_SLOTS: Record<1 | 2, readonly [number, number]> = { 1: [1, 2], 2: [3, 4] };

/** The most slots an osu! room has. */
const MAX_SLOTS = 16;

/** How many slots the room has, as far as anyone can tell. */
export function roomSize(room: Pick<RoomState, "size" | "slots">, atLeast = 0): number {
  return Math.min(MAX_SLOTS, Math.max(room.size ?? 0, atLeast, ...room.slots.map((s) => s.slot)));
}
