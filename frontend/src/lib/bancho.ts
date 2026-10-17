// Reads BanchoBot's messages in a multiplayer channel into the room's state, and
// builds the !mp commands the referee page sends. Message formats are BanchoBot's;
// anything unrecognised is just left in the chat.

export const BANCHO_BOT = "BanchoBot";

/** BanchoBot's line as a map starts. */
export const MATCH_STARTED = "The match has started!";

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
  /** Mods the player picked, with freemod on, as acronyms ("HDHR"); from !mp settings. */
  mods: string;
  /** Whether `mods` came from !mp settings; a player who just joined hasn't been read yet. */
  modsKnown: boolean;
}

export interface MapScore {
  username: string;
  /** Known when the score came from osu! rather than the chat, which only names players. */
  userId?: number;
  score: number;
  passed: boolean;
  /**
   * The mods the player played with, as acronyms ("HDHR"), the room's included; null when
   * unknown (the chat only knows them from the last !mp settings).
   */
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
  // Any status, so an unexpected one (while a map is played) doesn't drop the slot.
  slot: new RegExp(String.raw`^Slot (\d+) +([A-Za-z][A-Za-z ]*?) +${OSU}/u/(-?\d+) (.+?)(?: +\[(.*)\])?$`),
  joined: /^(.+) joined in slot (\d+)(?: for team (red|blue))?\.$/,
  moved: /^(.+) moved to slot (\d+)$/,
  changedTeam: /^(.+) changed to (Red|Blue)$/,
  left: /^(.+) left the game\.$/,
  kicked: /^Kicked (.+) from the match\.$/,
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

/** BanchoBot announcing someone leaving the room, or being kicked from it: their name. */
export function parseLeft(text: string): string | null {
  const m = RE.left.exec(text) ?? RE.kicked.exec(text);
  return m ? m[1] : null;
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

// BanchoBot's mod names, as osu! abbreviates them.
const MOD_ACRONYMS: Record<string, string> = {
  nofail: "NF",
  easy: "EZ",
  touchdevice: "TD",
  hidden: "HD",
  hardrock: "HR",
  suddendeath: "SD",
  doubletime: "DT",
  relax: "RX",
  halftime: "HT",
  nightcore: "NC",
  flashlight: "FL",
  spunout: "SO",
  relax2: "AP",
  autopilot: "AP",
  perfect: "PF",
  fadein: "FI",
  mirror: "MR",
  scorev2: "V2",
};

/** "Hidden, HardRock" (BanchoBot's names, freemod left out) as acronyms: "HDHR". */
export function modAcronyms(names: string): string {
  return names
    .split(",")
    .map((n) => n.trim().replace(/\s+/g, "").toLowerCase())
    .filter((n) => n !== "" && n !== "freemod" && n !== "none")
    .map((n) => MOD_ACRONYMS[n] ?? n.toUpperCase())
    .join("");
}

/** "HDHR" -> ["HD", "HR"]; "NM" and "" are no mods. */
export function splitMods(acronyms: string): string[] {
  return (acronyms.toUpperCase().match(/[A-Z0-9]{2}/g) ?? []).filter((m) => m !== "NM");
}

/** Whether the room's mods, as BanchoBot described them, include freemod. */
export function isFreemod(mods: string | null): boolean {
  return /free ?mod/i.test(mods ?? "");
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
      mods: modAcronyms(parts.filter((p) => p !== "Host" && p !== teamPart && p !== "").join(", ")),
      modsKnown: true,
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
      modsKnown: false,
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
  if ((m = RE.left.exec(text)) || (m = RE.kicked.exec(text))) {
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
  if ((m = RE.mods.exec(text))) {
    const mods = describeMods(text);
    // Without freemod, osu! takes the players' own mods off.
    const slots = isFreemod(mods) ? room.slots : room.slots.map((s) => ({ ...s, mods: "" }));
    return { ...room, mods, slots };
  }
  if (text === "Locked the match") return { ...room, locked: true };
  if (text === "Unlocked the match") return { ...room, locked: false };
  if (text === "All players are ready") {
    return { ...room, slots: room.slots.map((s) => ({ ...s, status: "ready" })) };
  }
  if (text === MATCH_STARTED) return { ...room, playing: true, current: [] };
  if ((m = RE.finished.exec(text))) {
    const score: MapScore = { username: m[1], score: Number(m[2]), passed: m[3] === "PASSED", mods: null };
    return { ...room, current: [...room.current.filter((s) => !sameName(s.username, score.username)), score] };
  }
  if (text === "The match has finished!") {
    const teams: Record<string, SlotTeam | null> = {};
    for (const s of room.slots) teams[normalizeName(s.username)] = s.team;
    const freemod = isFreemod(room.mods);
    const roomMods = modAcronyms(room.mods ?? "");
    // Each player's mods: the room's, plus their own with freemod as the last
    // !mp settings showed them (the referee page asks for it as each map starts).
    const scores = room.current.map((score) => {
      const slot = room.slots.find((s) => sameName(s.username, score.username));
      const own = freemod ? (slot?.modsKnown ? slot.mods : null) : "";
      return { ...score, mods: own === null ? null : roomMods + own };
    });
    const result: MapResult = {
      beatmapId: room.beatmapId,
      beatmapName: room.beatmapName,
      roomMods,
      freemod,
      scores,
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

// ---------- the chat as spectators read it ----------

// BanchoBot's lines a spectator follows the match by: maps set, started, finished,
// aborted, and the room closing. Everything else it says (joins, slot moves, settings
// dumps, timers, locks, mods, and each player's "finished playing", which the score
// sheet already lists) is the room being run.
const MATCH_LINES = [
  /^The match has (started|finished)!$/,
  /^Aborted the match$/,
  /^Closed the match$/,
  RE.changedBeatmap,
  RE.beatmapChanged,
  // Who picks first.
  /^.+ rolls \d+ point\(s\)$/,
];

/**
 * Whether a room line is the room being run rather than the match being played: BanchoBot's
 * housekeeping, and the referee's !mp commands. The public match page hides these unless asked.
 */
export function isRoomNoise(from: string, text: string): boolean {
  if (from === BANCHO_BOT) return !MATCH_LINES.some((re) => re.test(text));
  return /^!mp\b/i.test(text.trim());
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

// ---------- placing players ----------

/** The colour each side of a bracket match plays as in the room: team 1 blue, team 2 red. */
export const SIDE_COLOR: Record<1 | 2, SlotTeam> = { 1: "blue", 2: "red" };

/** The slots each side's two players sit in: team 1 (blue) in 1 and 2, team 2 (red) in 3 and 4. */
export const SIDE_SLOTS: Record<1 | 2, readonly [number, number]> = { 1: [1, 2], 2: [3, 4] };

/** The most slots an osu! room has. */
const MAX_SLOTS = 16;

/** A player in the room, addressed by osu! id when it's known and as BanchoBot names them otherwise. */
export function slotRef(slot: Pick<RoomSlot, "userId" | "username">): string {
  return slot.userId !== null ? playerRef(slot.userId) : ircName(slot.username);
}

/** How many slots the room has, as far as anyone can tell. */
export function roomSize(room: Pick<RoomState, "size" | "slots">, atLeast = 0): number {
  return Math.min(MAX_SLOTS, Math.max(room.size ?? 0, atLeast, ...room.slots.map((s) => s.slot)));
}

/** Who belongs in the room's match slots, for moving anyone else out of them. */
export interface MatchSlots {
  /** The slots the match's players sit in. */
  reserved: readonly number[];
  /** Whether someone in the room is one of the match's players. */
  isPlayer: (slot: Pick<RoomSlot, "username" | "userId">) => boolean;
}

/**
 * The !mp commands that put a player in the room into slot `to` and onto `team` (either null to
 * leave it as it is). A slot someone else holds is swapped: they wait in a free slot (the room
 * grows by one for them when it's full, and shrinks back after), the player takes the slot, and
 * they take the player's old one. Given `match`, someone in a match slot who isn't one of its
 * players is moved out to a free slot past the match's instead, and stays there.
 */
export function moveCommands(
  room: Pick<RoomState, "size" | "slots">,
  mover: RoomSlot,
  to: number | null,
  team: SlotTeam | null,
  match?: MatchSlots,
): string[] {
  const commands: string[] = [];
  if (to !== null && to !== mover.slot) {
    const other = room.slots.find((s) => s.slot === to && s !== mover && !sameName(s.username, mover.username));
    if (!other) {
      commands.push(`!mp move ${slotRef(mover)} ${to}`);
    } else {
      const outsider = match !== undefined && !match.isPlayer(other);
      // A room grown for an outsider grows past the match's slots, whatever size it was thought to be.
      const size = roomSize(room, outsider ? Math.max(to, ...match.reserved) : to);
      const taken = new Set(room.slots.map((s) => s.slot));
      let spare = Array.from({ length: size }, (_, i) => i + 1).find(
        (n) => !taken.has(n) && !(outsider && match.reserved.includes(n)),
      );
      const grow = spare === undefined && size < MAX_SLOTS;
      if (grow) {
        spare = size + 1;
        commands.push(`!mp size ${spare}`);
      }
      if (spare !== undefined) {
        commands.push(`!mp move ${slotRef(other)} ${spare}`, `!mp move ${slotRef(mover)} ${to}`);
        if (!outsider) {
          commands.push(`!mp move ${slotRef(other)} ${mover.slot}`);
          if (grow) commands.push(`!mp size ${size}`);
        }
      }
    }
  }
  if (team !== null && team !== mover.team) commands.push(`!mp team ${slotRef(mover)} ${team}`);
  return commands;
}

/** Someone allowed in the room: one of the match's players, or a host, referee or streamer. */
export interface RoomMember {
  osuId: number;
  username: string;
}

/**
 * Whether someone in the room is one of `members`: by osu! id once it's known (so a player who
 * renamed is still theirs), by name until then.
 */
export function isMember(slot: Pick<RoomSlot, "username" | "userId">, members: RoomMember[]): boolean {
  return members.some((m) => (slot.userId !== null ? slot.userId === m.osuId : sameName(slot.username, m.username)));
}

/** Removes someone from the room. */
export const kickCommand = (slot: Pick<RoomSlot, "userId" | "username">) => `!mp kick ${slotRef(slot)}`;

/** A bracket match's slots and players, for moveCommands and placeCommands. */
export function matchSlots(players: Pick<Placement, "osuId" | "username">[]): MatchSlots {
  return {
    reserved: [...SIDE_SLOTS[1], ...SIDE_SLOTS[2]],
    isPlayer: (s) => players.some((p) => s.userId === p.osuId || sameName(s.username, p.username)),
  };
}

/** Where a player belongs in the room. */
export interface Placement {
  osuId: number;
  username: string;
  /** Their slot; null when any will do. */
  slot: number | null;
  /** Their colour; null in head to head. */
  team: SlotTeam | null;
}

// The room after the !mp move, team and size commands moveCommands makes, as Bancho would leave it.
function afterCommands(room: Pick<RoomState, "size" | "slots">, commands: string[]): Pick<RoomState, "size" | "slots"> {
  let { size, slots } = room;
  for (const line of commands) {
    const [, verb, ref, value] = line.split(" ");
    if (verb === "size") {
      size = Number(ref);
      continue;
    }
    slots = slots.map((s) =>
      slotRef(s) !== ref ? s : verb === "move" ? { ...s, slot: Number(value) } : { ...s, team: value as SlotTeam },
    );
  }
  return { size, slots };
}

/**
 * The !mp commands that put every one of `players` who's in the room in their place, one after
 * the other, each move counted in for the next (see moveCommands).
 */
export function placeCommands(
  room: Pick<RoomState, "size" | "slots">,
  players: Placement[],
  match?: MatchSlots,
): string[] {
  // Players are addressed by osu! id, which the chat's joins don't give.
  let state = {
    size: room.size,
    slots: room.slots.map((s) => {
      const p = players.find((p) => s.userId === p.osuId || sameName(s.username, p.username));
      return p ? { ...s, userId: p.osuId } : s;
    }),
  };
  const commands: string[] = [];
  for (const p of players) {
    const mover = state.slots.find((s) => s.userId === p.osuId);
    if (!mover) continue;
    const next = moveCommands(state, mover, p.slot, p.team, match);
    commands.push(...next);
    state = afterCommands(state, next);
  }
  return commands;
}

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
