import type { PublicPoolMap, RefereeRoom } from "../../../api/types";
import { isFreemodCategory, splitMods, type MapResult, type SlotTeam } from "../../../lib/bancho";

// The match's pool as the referee page shows it, shared by the pool panel and the
// results.

/** How many bans a bracket match has; the ban buttons give way to the pick buttons after them. */
export const BAN_LIMIT = 2;

export interface PoolEntry {
  /** "NM1", "HD2", "TB". */
  label: string;
  /** The category's colour, "#rrggbb". */
  color: string;
  freemod: boolean;
  /** A tiebreaker: nobody picks it. */
  tiebreaker: boolean;
  map: PublicPoolMap;
}

export interface MatchTeam {
  id: string;
  name: string;
  color: SlotTeam;
}

/** The pool in category and slot order. A category with one slot is just its name. */
export function poolEntries(room: RefereeRoom): PoolEntry[] {
  const entries: PoolEntry[] = [];
  for (const category of room.categories) {
    const slots = room.slots.filter((s) => s.category_id === category.id).sort((a, b) => a.position - b.position);
    slots.forEach((slot, i) => {
      const map = room.maps.find((m) => m.slot_id === slot.id);
      if (!map) return;
      entries.push({
        label: slots.length > 1 ? `${category.name}${i + 1}` : category.name,
        color: category.color,
        freemod: isFreemodCategory(category.name),
        tiebreaker: /^TB/i.test(category.name.trim()),
        map,
      });
    });
  }
  return entries;
}

/** A bracket match's two teams, red then blue, once both are known; otherwise none. */
export function matchTeams(room: RefereeRoom): MatchTeam[] {
  if (room.kind !== "bracket") return [];
  const team = (id: string | null, color: SlotTeam): MatchTeam | null => {
    const t = id ? room.teams.find((t) => t.id === id) : undefined;
    return t ? { id: t.id, name: t.name, color } : null;
  };
  const both = [team(room.team1_id, "red"), team(room.team2_id, "blue")];
  return both.every((t) => t !== null) ? (both as MatchTeam[]) : [];
}

// Mods compared without NoFail (the referee adds it to everything), Nightcore as the
// DoubleTime it is, in a fixed order.
function modKey(acronyms: string): string {
  return splitMods(acronyms)
    .filter((m) => m !== "NF")
    .map((m) => (m === "NC" ? "DT" : m))
    .sort()
    .join("");
}

/**
 * The pool entry a finished map was: the one with its beatmap. When the pool has that beatmap
 * more than once (under different mods, or as both FM and TB), the one whose mods it was played
 * with, then the one a team picked, then the first.
 */
export function entryOf(
  entries: PoolEntry[],
  result: Pick<MapResult, "beatmapId" | "roomMods" | "freemod">,
  picked: Set<string>,
): PoolEntry | undefined {
  const candidates = entries.filter((e) => e.map.beatmap_id === result.beatmapId);
  if (candidates.length <= 1) return candidates[0];
  const played = modKey(result.roomMods);
  const byMods = candidates.filter((e) =>
    result.freemod ? e.freemod : !e.freemod && modKey(e.map.mods) === played,
  );
  const pool = byMods.length > 0 ? byMods : candidates;
  return pool.find((e) => picked.has(e.map.id)) ?? pool[0];
}

/**
 * The team whose turn it is to pick, by the rules: the roll's winner picks first and bans second,
 * so the first pick goes to the team that didn't ban first, and picks alternate after that. Null
 * before anything is marked, when the page can't know who won the roll.
 */
export function nextPicker(room: RefereeRoom, teams: MatchTeam[]): MatchTeam | null {
  if (teams.length !== 2) return null;
  const last = room.picks[room.picks.length - 1] ?? room.bans[0];
  return last ? (teams.find((t) => t.id !== last.team_id) ?? null) : null;
}

/** The line the referee sends to the room: "Skor: Team A 2 - 1 Team B | Sıradaki pick: Team B". */
export function scoreMessage(teams: MatchTeam[], score: [number, number], next: MatchTeam | null): string {
  const line = `Skor: ${teams[0].name} ${score[0]} - ${score[1]} ${teams[1].name}`;
  return next ? `${line} | Sıradaki pick: ${next.name}` : line;
}
