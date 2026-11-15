import type { BracketMatch, BracketSide, Lobby, MatchStage, Player } from "../api/types";

/** W3, L1, … */
export function matchCode(m: Pick<BracketMatch, "side" | "number">): string {
  return `${m.side === "winners" ? "W" : "L"}${m.number}`;
}

/** Opens the bracket's grid on a match (`?match=`, read by `useLinkedMatch`): the admin bracket or the public one. */
export function bracketMatchPath(matchId: string, admin: boolean): string {
  return `${admin ? "/admin/bracket" : "/bracket"}?match=${matchId}`;
}

/** A match's public page, which follows it live (pages/public/match/MatchPage.tsx). */
export function matchPagePath(matchId: string): string {
  return `/match/${matchId}`;
}

/**
 * Whether a bracket match is linked to its public page: once both its teams are known. A potential match
 * has none until it's decided.
 */
export function hasMatchPage(
  m: Pick<BracketMatch, "team1_id" | "team2_id" | "potential_of">,
): boolean {
  return m.potential_of === undefined && m.team1_id !== null && m.team2_id !== null;
}

/**
 * Where a fixture row leads: the match's own page, or the bracket while it's still waiting for a team (its
 * bracket match, for a potential match).
 */
export function fixturePath(
  m: Pick<BracketMatch, "id" | "team1_id" | "team2_id" | "potential_of">,
): string {
  if (m.potential_of !== undefined) return bracketMatchPath(m.potential_of, false);
  return hasMatchPage(m) ? matchPagePath(m.id) : bracketMatchPath(m.id, false);
}

/** What a match still waiting on its pairings shows in place of its time. */
export const UNDECIDED_TIME = "Daha kesinleşmedi";

/** Whether a match is waiting to find out which of its potential matches it will be. */
export function isUndecided(m: BracketMatch): boolean {
  return m.potentials.length > 0;
}

/**
 * A bracket match's potential matches as matches of their own, for the views that list what's to be played
 * (the list, the calendar, the fixture rows): the bracket match's code, side and stage with each pairing's
 * id, teams, time and staff, and `potential_of` pointing back at it. They have no room, score or links.
 */
export function potentialRows<M extends BracketMatch>(m: M): M[] {
  return m.potentials.map((p) => ({
    ...m,
    id: p.id,
    team1_id: p.team1_id,
    team2_id: p.team2_id,
    starts_at: p.starts_at,
    referee: p.referee,
    streamer: p.streamer,
    team1_score: null,
    team2_score: null,
    forfeit_slot: null,
    rooms: [],
    concluded_at: null,
    winner_to: null,
    loser_to: null,
    potentials: [],
    potential_of: m.id,
  }));
}

/** The matches with each undecided one replaced by its potential matches. */
export function withPotentials<M extends BracketMatch>(matches: M[]): M[] {
  return matches.flatMap((m) => (isUndecided(m) ? potentialRows(m) : [m]));
}

/** The match a row stands for: a potential match's bracket match, otherwise itself. */
export function rowMatchId(m: Pick<BracketMatch, "id" | "potential_of">): string {
  return m.potential_of ?? m.id;
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
export function incomingSources<M extends BracketMatch>(
  matches: M[],
): Map<string, [Source<M> | null, Source<M> | null]> {
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
  if (!m.concluded_at) return null;
  if (m.forfeit_slot !== null) return m.forfeit_slot === 1 ? 2 : 1;
  if (m.team1_score === null || m.team2_score === null) return null;
  if (m.team1_score === m.team2_score) return null;
  return m.team1_score > m.team2_score ? 1 : 2;
}

/**
 * Whether a match is being played right now: its time has come, a referee has opened its room
 * and it isn't concluded yet. Its score, while it's live, is the running one.
 */
export function isLive(m: BracketMatch, now = Date.now()): boolean {
  return (
    !m.concluded_at &&
    m.starts_at !== null &&
    new Date(m.starts_at).getTime() <= now &&
    m.rooms.length > 0
  );
}

/** A qualifier lobby being played: its time has come, it has a room and it isn't concluded. */
export function isLobbyLive(lobby: Lobby<Player>): boolean {
  return !lobby.open && !lobby.concluded_at && lobby.rooms.length > 0;
}

/** The two halves of a match's score as shown: the figures, or W and FF for a forfeit, or null without one. */
export function scoreHalves(m: BracketMatch): [string, string] | null {
  if (m.concluded_at && m.forfeit_slot !== null)
    return m.forfeit_slot === 1 ? ["FF", "W"] : ["W", "FF"];
  if (m.team1_score === null || m.team2_score === null) return null;
  return [String(m.team1_score), String(m.team2_score)];
}

/** The matches with one match's score replaced, as a live score event brings it. */
export function withScore<M extends BracketMatch>(
  matches: M[],
  matchId: string,
  team1Score: number | null,
  team2Score: number | null,
): M[] {
  return matches.map((m) =>
    m.id === matchId ? { ...m, team1_score: team1Score, team2_score: team2Score } : m,
  );
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

export interface StageGroup<M extends BracketMatch> {
  /** null for the matches that have no stage yet. */
  stage: MatchStage | null;
  matches: M[];
}

/** Matches by stage in stage order, the ones without a stage last; each group in time order. */
export function groupByStage<M extends BracketMatch>(matches: M[]): StageGroup<M>[] {
  const groups = new Map<string | null, StageGroup<M>>();
  for (const m of [...matches].sort(compareByTime)) {
    const key = m.stage?.id ?? null;
    const group = groups.get(key) ?? { stage: m.stage, matches: [] };
    group.matches.push(m);
    groups.set(key, group);
  }
  const position = (g: StageGroup<M>) => g.stage?.position ?? Infinity;
  return [...groups.values()].sort((a, b) => position(a) - position(b));
}

/**
 * The bracket stage being played now: the stage of the earliest unfinished match (by time, then code), as the
 * stage track decides it for bracket stages (backend `stage_track.rs`); matches without a stage don't count.
 * null once every match with a stage is over, or before any has one.
 */
export function currentStageId(matches: BracketMatch[]): string | null {
  const next = matches.filter((m) => m.stage && !m.concluded_at).sort(compareByTime)[0];
  return next?.stage?.id ?? null;
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
