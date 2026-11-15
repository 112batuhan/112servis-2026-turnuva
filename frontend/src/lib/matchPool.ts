import type { PublicMatch, PublicPoolMap, RefereeRoom } from "../api/types";
import { isFreemodCategory, SIDE_COLOR, type SlotTeam } from "./bancho";
import { draftOf, stageDraftOrder, stepLabel, type Draft, type StepKind } from "./draftOrder";

// The match's pool as the referee page shows it, shared by the pool panel and the
// results, and by the public match page, which shows the same match.

/** A match as the referee page or the public match page has it: what both share. */
export type MatchData = Pick<
  RefereeRoom | PublicMatch,
  | "id"
  | "kind"
  | "categories"
  | "slots"
  | "maps"
  | "team1_id"
  | "team2_id"
  | "team1_score"
  | "team2_score"
  | "forfeit_slot"
  | "concluded_at"
  | "bans"
  | "picks"
  | "manual_results"
  | "map_results"
  | "rooms"
  | "stage"
> & { teams: { id: string; name: string }[] };

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

/** The next ban or pick and the team it falls to (null until the first mark says who 1 is). */
export interface DraftTurn {
  kind: StepKind;
  team: MatchTeam | null;
}

/** The pool in category and slot order. A category with one slot is just its name. */
export function poolEntries(room: Pick<MatchData, "categories" | "slots" | "maps">): PoolEntry[] {
  const entries: PoolEntry[] = [];
  for (const category of room.categories) {
    const slots = room.slots
      .filter((s) => s.category_id === category.id)
      .sort((a, b) => a.position - b.position);
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

/** A bracket match's two teams, blue (team 1) then red, once both are known; otherwise none. */
export function matchTeams(
  room: Pick<MatchData, "kind" | "teams" | "team1_id" | "team2_id">,
): MatchTeam[] {
  if (room.kind !== "bracket") return [];
  const team = (id: string | null, color: SlotTeam): MatchTeam | null => {
    const t = id ? room.teams.find((t) => t.id === id) : undefined;
    return t ? { id: t.id, name: t.name, color } : null;
  };
  const both = [team(room.team1_id, SIDE_COLOR[1]), team(room.team2_id, SIDE_COLOR[2])];
  return both.every((t) => t !== null) ? (both as MatchTeam[]) : [];
}

/**
 * A bracket match's draft by its stage's order, once both teams are known. `chosen` is the team
 * the referee said goes next; `open` while there's a ban or pick to make (not at the tiebreaker,
 * nor once a team has won or the match is over).
 */
export function matchDraft(room: MatchData, chosen: string | null, open: boolean): Draft | null {
  const teams = matchTeams(room);
  if (teams.length !== 2) return null;
  const order = stageDraftOrder(room.stage?.draft_order);
  return draftOf(order, room.bans, room.picks, [teams[0].id, teams[1].id], chosen, open);
}

/** The draft's next ban or pick with its team, for the pages that name it. */
export function draftTurn(draft: Draft | null, teams: MatchTeam[]): DraftTurn | null {
  const next = draft?.next;
  return next ? { kind: next.kind, team: teams.find((t) => t.id === next.teamId) ?? null } : null;
}

/**
 * The line the room's bot sends for "Skoru odaya yaz": "Skor: Team A 2 - 1 Team B | Sıradaki pick: Team B"
 * (or "Sıradaki ban"). The server builds the one it sends the same way (bancho::tally::score_message);
 * this is its preview.
 */
export function scoreMessage(
  teams: MatchTeam[],
  score: [number, number],
  next: DraftTurn | null,
): string {
  const line = `Skor: ${teams[0].name} ${score[0]} - ${score[1]} ${teams[1].name}`;
  return next?.team
    ? `${line} | Sıradaki ${stepLabel(next.kind).toLowerCase()}: ${next.team.name}`
    : line;
}
