import type { PublicPoolMap, RefereeRoom } from "../../../api/types";
import { isFreemodCategory, type SlotTeam } from "../../../lib/bancho";

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
