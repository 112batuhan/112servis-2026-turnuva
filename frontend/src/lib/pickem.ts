// Pick'em: predicting each bracket match of a stage before it starts (handlers/pickem.rs and
// backend/src/pickem.rs hold the rules; the server scores, this only shows them).
import type {
  BracketMatch,
  MyPrediction,
  PickemScoreCount,
  PickemStage,
  PickemStatus,
} from "../api/types";
import { firstTo } from "./bestOf";

/** The pick'em page opened on a match: its stage shown and its score distribution open. */
export function pickemMatchPath(matchId: string): string {
  return `/pickem?match=${matchId}`;
}

/** Both right, the winner only, the score only. */
export const POINTS = { exact: 3, winner: 2, score: 1 } as const;

/** What a prediction's points mean, said in a word. */
export function pointsLabel(points: number): string {
  if (points === POINTS.exact) return "Tam isabet";
  if (points === POINTS.winner) return "Kazanan doğru";
  if (points === POINTS.score) return "Skor doğru";
  return "Tutmadı";
}

export const STATUS_LABELS: Record<PickemStatus, string> = {
  upcoming: "Bekliyor",
  open: "Tahmine açık",
  locked: "Kilitli",
  finished: "Bitti",
};

/** Whether a stage's predictions are past changing, so their split is public. */
export function isLocked(status: PickemStatus): boolean {
  return status === "locked" || status === "finished";
}

/**
 * Every result a match can end on, as the room's sides read it (team 1 first), from team 1's
 * biggest win through the closest results to team 2's biggest: for a Bo9 5–0 … 5–4, 4–5 … 0–5.
 * The ballot's chips and the revealed histogram share this axis.
 */
export interface Outcome {
  slot: 1 | 2;
  winner: number;
  loser: number;
  /** The score as the sides read it. */
  team1: number;
  team2: number;
}

export function outcomes(bestOf: number): Outcome[] {
  const need = firstTo(bestOf);
  const out: Outcome[] = [];
  for (let loser = 0; loser < need; loser++)
    out.push({ slot: 1, winner: need, loser, team1: need, team2: loser });
  for (let loser = need - 1; loser >= 0; loser--)
    out.push({ slot: 2, winner: need, loser, team1: loser, team2: need });
  return out;
}

export const outcomeKey = (o: { slot: 1 | 2; loser: number }) => `${o.slot}:${o.loser}`;

/** The slot a prediction is on in this match, or null when its team is no longer in it. */
export function predictionSlot(m: BracketMatch, p: Pick<MyPrediction, "team_id">): 1 | 2 | null {
  if (p.team_id === m.team1_id) return 1;
  if (p.team_id === m.team2_id) return 2;
  return null;
}

/** A prediction's score as the sides read it: [team 1, team 2]. */
export function predictionScore(
  slot: 1 | 2,
  p: Pick<MyPrediction, "winner_score" | "loser_score">,
): [number, number] {
  return slot === 1 ? [p.winner_score, p.loser_score] : [p.loser_score, p.winner_score];
}

/** Two whole percentages that add up to 100 (none when nobody predicted). */
export function shares(a: number, b: number): [number, number] {
  const total = a + b;
  if (total === 0) return [0, 0];
  const first = Math.round((a / total) * 100);
  return [first, 100 - first];
}

/** How many predicted each outcome. */
export function countsByOutcome(scores: PickemScoreCount[]): Map<string, number> {
  return new Map(scores.map((s) => [outcomeKey({ slot: s.slot, loser: s.loser_score }), s.count]));
}

/** Whether a match has begun: its time has come, it has a room or it's over (pickem::started). */
export function matchStarted(m: BracketMatch, now = Date.now()): boolean {
  return (
    !!m.concluded_at ||
    m.rooms.length > 0 ||
    (m.starts_at !== null && new Date(m.starts_at).getTime() <= now)
  );
}

/**
 * Whether a match takes no more predictions (pickem::match_locked): once it has begun, except
 * that in a stage a host opened the clock is overridden, and only its room or its result locks it.
 */
export function matchClosed(
  stage: Pick<PickemStage, "mode">,
  m: BracketMatch,
  now = Date.now(),
): boolean {
  return stage.mode === "open" ? !!m.concluded_at || m.rooms.length > 0 : matchStarted(m, now);
}

/**
 * Whether a match's predictions are past changing, so its split is public: its stage is locked, or
 * it's locked on its own in a stage a host opened.
 */
export function isMatchLocked(
  stage: Pick<PickemStage, "status" | "mode">,
  m: BracketMatch,
  now = Date.now(),
): boolean {
  return isLocked(stage.status) || (stage.status === "open" && matchClosed(stage, m, now));
}

/** Whether a match can be predicted now: the server checks the same. */
export function canPredictMatch(
  stage: Pick<PickemStage, "status" | "mode" | "best_of">,
  m: BracketMatch,
  now = Date.now(),
): boolean {
  return (
    stage.status === "open" &&
    stage.best_of !== null &&
    m.team1_id !== null &&
    m.team2_id !== null &&
    !matchClosed(stage, m, now)
  );
}

/** The stage the page opens on: the open one, else the last that locked, else the first. */
export function defaultStage(stages: PickemStage[]): PickemStage | undefined {
  return (
    stages.find((s) => s.status === "open") ??
    [...stages].reverse().find((s) => isLocked(s.status)) ??
    stages[0]
  );
}
