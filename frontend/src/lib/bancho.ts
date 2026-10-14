// Reads BanchoBot's messages in a multiplayer channel into the room's state, and
// builds the !mp commands the referee page sends. Message formats are BanchoBot's;
// anything unrecognised is just left in the chat.

export const BANCHO_BOT = "BanchoBot";

/** Prefixed to every room name: "112S: (Team A) vs (Team B)". */
export const ROOM_PREFIX = "112S";

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
  /** Mods the player picked, with freemod on (from !mp settings). */
  mods: string;
}

export interface MapScore {
  username: string;
  score: number;
  passed: boolean;
}

export interface MapResult {
  beatmapId: number | null;
  beatmapName: string | null;
  scores: MapScore[];
  /** Each player's team when the map finished. */
  teams: Record<string, SlotTeam | null>;
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
  /** Finished maps, oldest first. */
  results: MapResult[];
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
  results: [],
};

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

const OSU = String.raw`https://osu\.ppy\.sh`;
const RE = {
  created: new RegExp(String.raw`^Created the tournament match ${OSU}/mp/(\d+) (.*)$`),
  roomName: new RegExp(String.raw`^Room name: (.*), History: ${OSU}/mp/(\d+)$`),
  beatmap: new RegExp(String.raw`^Beatmap: ${OSU}/b/(\d+) (.*)$`),
  teamMode: /^Team mode: (\w+), Win condition: (\w+)$/,
  activeMods: /^Active mods: (.*)$/,
  players: /^Players: (\d+)$/,
  slot: new RegExp(String.raw`^Slot (\d+) +(Ready|Not Ready|No Map) +${OSU}/u/(-?\d+) (.+?)(?: +\[(.*)\])?$`),
  joined: /^(.+) joined in slot (\d+)(?: for team (red|blue))?\.$/,
  moved: /^(.+) moved to slot (\d+)$/,
  changedTeam: /^(.+) changed to (Red|Blue)$/,
  left: /^(.+) left the game\.$/,
  becameHost: /^(.+) became the host\.$/,
  changedHost: /^Changed match host to (.+)$/,
  changedBeatmap: new RegExp(String.raw`^Changed beatmap to ${OSU}/b/(\d+) (.*)$`),
  beatmapChanged: new RegExp(String.raw`^Beatmap changed to: (.*) \(${OSU}/b/(\d+)\)$`),
  settings: /^Changed match settings to (?:(\d+) slots, )?(\w+), (\w+)$/,
  size: /^Changed match to size (\d+)$/,
  finished: /^(.+) finished playing \(Score: (\d+), (PASSED|FAILED)\)\.$/,
  mods: /^(Enabled|Disabled) (.*)$/,
};

/** The room id from BanchoBot's reply to !mp make, or null for any other message. */
export function parseCreated(text: string): { mpId: number; name: string } | null {
  const m = RE.created.exec(text);
  return m ? { mpId: Number(m[1]), name: m[2] } : null;
}

/** BanchoBot announcing a player joining the room. */
export function parseJoined(text: string): { username: string; slot: number; team: SlotTeam | null } | null {
  const m = RE.joined.exec(text);
  return m ? { username: m[1], slot: Number(m[2]), team: (m[3] as SlotTeam | undefined) ?? null } : null;
}

function withSlot(room: RoomState, username: string, update: (slot: RoomSlot) => RoomSlot): RoomState {
  return {
    ...room,
    slots: room.slots.map((s) => (sameName(s.username, username) ? update(s) : s)),
  };
}

function sortSlots(slots: RoomSlot[]): RoomSlot[] {
  return [...slots].sort((a, b) => a.slot - b.slot);
}

// Bancho unreadies everyone when the map changes.
function changeBeatmap(room: RoomState, beatmapId: number, beatmapName: string): RoomState {
  return { ...room, beatmapId, beatmapName, slots: room.slots.map((s) => ({ ...s, status: "not_ready" })) };
}

// "Enabled Hidden, NoFail, disabled FreeMod" / "Disabled all mods, enabled FreeMod"
function describeMods(text: string): string {
  if (/enabled FreeMod/i.test(text)) return "Freemod";
  const enabled = /^Enabled (.*?)(?:, disabled .*)?$/.exec(text);
  return enabled ? enabled[1] : "";
}

/** Applies one BanchoBot line from the room's channel. */
export function applyBancho(room: RoomState, text: string): RoomState {
  let m: RegExpExecArray | null;

  // !mp settings: a snapshot, so the slot list starts over at "Players: N".
  if ((m = RE.roomName.exec(text))) return { ...room, name: m[1] };
  if ((m = RE.beatmap.exec(text))) return { ...room, beatmapId: Number(m[1]), beatmapName: m[2] };
  if ((m = RE.teamMode.exec(text))) return { ...room, teamMode: m[1], winCondition: m[2] };
  if ((m = RE.activeMods.exec(text))) return { ...room, mods: m[1] };
  if ((m = RE.players.exec(text))) return { ...room, slots: [] };
  if ((m = RE.slot.exec(text))) {
    const [, slot, status, userId, name, extra] = m;
    const parts = (extra ?? "").split(" / ").map((p) => p.trim());
    const teamPart = parts.find((p) => p.startsWith("Team "));
    const entry: RoomSlot = {
      slot: Number(slot),
      username: name.trim(),
      userId: Number(userId),
      team: teamPart ? (teamPart.slice(5).toLowerCase() as SlotTeam) : null,
      status: status === "Ready" ? "ready" : status === "No Map" ? "no_map" : "not_ready",
      host: parts.includes("Host"),
      mods: parts.filter((p) => p !== "Host" && p !== teamPart && p !== "").join(", "),
    };
    return {
      ...room,
      slots: sortSlots([...room.slots.filter((s) => s.slot !== entry.slot && s.userId !== entry.userId), entry]),
    };
  }

  // Live events.
  if ((m = RE.joined.exec(text))) {
    const [, name, slot, team] = m;
    const known = room.slots.find((s) => sameName(s.username, name));
    const entry: RoomSlot = {
      slot: Number(slot),
      username: name,
      userId: known?.userId ?? null,
      team: (team as SlotTeam | undefined) ?? null,
      status: "not_ready",
      host: false,
      mods: "",
    };
    return {
      ...room,
      slots: sortSlots([
        ...room.slots.filter((s) => s.slot !== entry.slot && !sameName(s.username, name)),
        entry,
      ]),
    };
  }
  if ((m = RE.moved.exec(text))) {
    const slot = Number(m[2]);
    return { ...room, slots: sortSlots(withSlot(room, m[1], (s) => ({ ...s, slot })).slots) };
  }
  if ((m = RE.changedTeam.exec(text))) {
    const team = m[2].toLowerCase() as SlotTeam;
    return withSlot(room, m[1], (s) => ({ ...s, team }));
  }
  if ((m = RE.left.exec(text))) {
    const name = m[1];
    return { ...room, slots: room.slots.filter((s) => !sameName(s.username, name)) };
  }
  if ((m = RE.becameHost.exec(text)) || (m = RE.changedHost.exec(text))) {
    const name = m[1];
    return { ...room, slots: room.slots.map((s) => ({ ...s, host: sameName(s.username, name) })) };
  }
  if (text === "Cleared match host") return { ...room, slots: room.slots.map((s) => ({ ...s, host: false })) };
  if ((m = RE.changedBeatmap.exec(text))) return changeBeatmap(room, Number(m[1]), m[2]);
  if ((m = RE.beatmapChanged.exec(text))) return changeBeatmap(room, Number(m[2]), m[1]);
  if ((m = RE.settings.exec(text))) {
    return { ...room, size: m[1] ? Number(m[1]) : room.size, teamMode: m[2], winCondition: m[3] };
  }
  if ((m = RE.size.exec(text))) return { ...room, size: Number(m[1]) };
  if ((m = RE.mods.exec(text))) return { ...room, mods: describeMods(text) };
  if (text === "Locked the match") return { ...room, locked: true };
  if (text === "Unlocked the match") return { ...room, locked: false };
  if (text === "All players are ready") {
    return { ...room, slots: room.slots.map((s) => ({ ...s, status: "ready" })) };
  }
  if (text === "The match has started!") return { ...room, playing: true, current: [] };
  if ((m = RE.finished.exec(text))) {
    const score: MapScore = { username: m[1], score: Number(m[2]), passed: m[3] === "PASSED" };
    return { ...room, current: [...room.current.filter((s) => !sameName(s.username, score.username)), score] };
  }
  if (text === "The match has finished!") {
    const teams: Record<string, SlotTeam | null> = {};
    for (const s of room.slots) teams[normalizeName(s.username)] = s.team;
    const result: MapResult = {
      beatmapId: room.beatmapId,
      beatmapName: room.beatmapName,
      scores: room.current,
      teams,
    };
    return {
      ...room,
      playing: false,
      current: [],
      results: [...room.results, result],
      slots: room.slots.map((s) => ({ ...s, status: "not_ready" })),
    };
  }
  if (text === "Aborted the match") return { ...room, playing: false, current: [] };
  if (text === "Closed the match") return { ...room, closed: true, playing: false };
  return room;
}

// ---------- commands ----------

export type ScoreMode = 0 | 1 | 2 | 3;
export const SCORE_MODES: { value: ScoreMode; label: string }[] = [
  { value: 3, label: "Score V2" },
  { value: 0, label: "Score" },
  { value: 1, label: "Accuracy" },
  { value: 2, label: "Combo" },
];

/** !mp set's team modes. */
export const TEAM_MODE = { headToHead: 0, teamVs: 2 } as const;

/** "#<osu id>" addresses a player in !mp commands whatever their current name. */
export const playerRef = (osuId: number) => `#${osuId}`;

/** A pool category is freemod when it's named FM or TB (FM1, TB, …). */
export function isFreemodCategory(name: string): boolean {
  return /^(FM|TB)/i.test(name.trim());
}

/**
 * The !mp mods line for a pool map. "HDDT" -> "!mp mods HD DT NF". Freemod maps get
 * just "Freemod": players pick their own mods there, NoFail included.
 */
export function modsCommand(mods: string, freemod: boolean, noFail: boolean): string {
  const list = freemod ? ["Freemod"] : (mods.toUpperCase().match(/[A-Z]{2}/g) ?? []).filter((m) => m !== "NM");
  if (noFail && !freemod && !list.includes("NF")) list.push("NF");
  return `!mp mods ${list.length > 0 ? list.join(" ") : "None"}`;
}
