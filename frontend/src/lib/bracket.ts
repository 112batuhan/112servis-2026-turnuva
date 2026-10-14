import type { BracketMatch, BracketSide } from "../api/types";

/** W3, L1, … */
export function matchCode(m: Pick<BracketMatch, "side" | "number">): string {
  return `${m.side === "winners" ? "W" : "L"}${m.number}`;
}

export const SIDE_LABELS: Record<BracketSide, string> = {
  winners: "Winners",
  losers: "Losers",
};

export type Outcome = "winner" | "loser";

/** A match whose winner or loser fills a slot. */
export interface Source<M extends BracketMatch = BracketMatch> {
  match: M;
  outcome: Outcome;
}

export const OUTCOME_LABELS: Record<Outcome, string> = { winner: "kazananı", loser: "kaybedeni" };

/** "W1 kazananı" */
export function sourceLabel(source: Source): string {
  return `${matchCode(source.match)} ${OUTCOME_LABELS[source.outcome]}`;
}

/** For each match id, the sources feeding its slot 1 and slot 2. */
export function incomingSources<M extends BracketMatch>(matches: M[]): Map<string, [Source<M> | null, Source<M> | null]> {
  const map = new Map<string, [Source<M> | null, Source<M> | null]>();
  for (const m of matches) map.set(m.id, [null, null]);
  for (const match of matches) {
    for (const outcome of ["winner", "loser"] as const) {
      const next = outcome === "winner" ? match.winner_to : match.loser_to;
      const slots = next && map.get(next.match_id);
      if (slots) slots[next.slot - 1] = { match, outcome };
    }
  }
  return map;
}

/** 1 for team 1, 2 for team 2, null while undecided. */
export function winningSlot(m: BracketMatch): 1 | 2 | null {
  if (!m.concluded_at || m.team1_score === null || m.team2_score === null) return null;
  if (m.team1_score === m.team2_score) return null;
  return m.team1_score > m.team2_score ? 1 : 2;
}

/** The number a new match on `side` gets: one after the highest. */
export function nextNumber(matches: BracketMatch[], side: BracketSide): number {
  return Math.max(0, ...matches.filter((m) => m.side === side).map((m) => m.number)) + 1;
}

/** Scheduled matches by time, then unscheduled ones; ties by side and number. */
export function compareByTime(a: BracketMatch, b: BracketMatch): number {
  if (a.starts_at !== b.starts_at) {
    if (a.starts_at === null) return 1;
    if (b.starts_at === null) return -1;
    return a.starts_at < b.starts_at ? -1 : 1;
  }
  return compareByCode(a, b);
}

/** W1, W2, …, then L1, L2, … */
export function compareByCode(a: BracketMatch, b: BracketMatch): number {
  if (a.side !== b.side) return a.side === "winners" ? -1 : 1;
  return a.number - b.number;
}

// The editor's board. Positions are stored in board units so cards snap to a grid;
// every card is the same size.
export const BOARD = {
  /** Pixels per board unit. */
  unit: 20,
  cardW: 12,
  cardH: 5,
  /** The spacing "Maç ekle" lays new matches out on: a card plus a gap. */
  pitchX: 15,
  pitchY: 6,
};

export interface Point {
  x: number;
  y: number;
}

/** Whether a card at `p` would overlap another match's card. Touching is fine. */
export function overlapsAny(matches: BracketMatch[], p: Point, exceptId?: string): boolean {
  return matches.some(
    (m) =>
      m.id !== exceptId &&
      p.x < m.pos_x + BOARD.cardW &&
      m.pos_x < p.x + BOARD.cardW &&
      p.y < m.pos_y + BOARD.cardH &&
      m.pos_y < p.y + BOARD.cardH,
  );
}

/** The first free spot on the layout spacing, filling columns top to bottom from the left. */
export function firstFreeSpot(matches: BracketMatch[], rows = 8): Point {
  for (let col = 0; ; col++) {
    for (let row = 0; row < rows; row++) {
      const p = { x: col * BOARD.pitchX, y: row * BOARD.pitchY };
      if (!overlapsAny(matches, p)) return p;
    }
  }
}

/** Snaps a board position to whole units. The board has no edges, so it can be negative. */
export function snapPosition(n: number): number {
  return Math.round(n);
}
