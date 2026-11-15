import { describe, expect, it } from "vitest";
import type { BracketMatch } from "../api/types";
import {
  canPredictMatch,
  countsByOutcome,
  defaultStage,
  isMatchLocked,
  matchClosed,
  matchStarted,
  outcomeKey,
  outcomes,
  POINTS,
  pointsLabel,
  predictionScore,
  predictionSlot,
  shares,
} from "./pickem";

// What the page shows of the pick'em rules, which are the server's (backend/src/pickem.rs):
// `matchStarted` mirrors `started`, `matchClosed` `match_locked`, `canPredictMatch` `predictable`,
// `isMatchLocked` `revealed`.

const NOW = Date.parse("2026-11-10T12:00:00Z");
const at = (minutes: number) => new Date(NOW + minutes * 60_000).toISOString();

/** A bracket match with both teams that starts in `minutes`. */
function match(minutes: number | null, over: Partial<BracketMatch> = {}): BracketMatch {
  return {
    id: "m",
    team1_id: "t1",
    team2_id: "t2",
    starts_at: minutes === null ? null : at(minutes),
    rooms: [],
    concluded_at: null,
    ...over,
  } as BracketMatch;
}

describe("outcomes", () => {
  it("runs from team 1's biggest win through the closest results to team 2's biggest", () => {
    expect(outcomes(9).map((o) => `${o.team1}-${o.team2}`)).toEqual([
      "5-0",
      "5-1",
      "5-2",
      "5-3",
      "5-4",
      "4-5",
      "3-5",
      "2-5",
      "1-5",
      "0-5",
    ]);
    expect(outcomes(1).map((o) => `${o.team1}-${o.team2}`)).toEqual(["1-0", "0-1"]);
  });

  it("keys a score count like the outcome it's for", () => {
    const counts = countsByOutcome([{ slot: 2, winner_score: 5, loser_score: 3, count: 7 }]);
    const o = outcomes(9).find((o) => o.team1 === 3 && o.team2 === 5)!;
    expect(counts.get(outcomeKey(o))).toBe(7);
  });
});

describe("a prediction", () => {
  it("is on the slot its team is in, or on none once the team is out of the match", () => {
    expect(predictionSlot(match(60), { team_id: "t2" })).toBe(2);
    expect(predictionSlot(match(60), { team_id: "gone" })).toBeNull();
  });

  it("reads its score as the sides do", () => {
    expect(predictionScore(1, { winner_score: 5, loser_score: 3 })).toEqual([5, 3]);
    expect(predictionScore(2, { winner_score: 5, loser_score: 3 })).toEqual([3, 5]);
  });

  it("scores as the server does", () => {
    expect(POINTS).toEqual({ exact: 3, winner: 2, score: 1 });
    expect([3, 2, 1, 0].map(pointsLabel)).toEqual([
      "Tam isabet",
      "Kazanan doğru",
      "Skor doğru",
      "Tutmadı",
    ]);
  });
});

describe("shares", () => {
  it("are whole percentages that add up to 100", () => {
    expect(shares(1, 2)).toEqual([33, 67]);
    expect(shares(5, 0)).toEqual([100, 0]);
    expect(shares(0, 0)).toEqual([0, 0]);
  });
});

describe("a match's window", () => {
  it("starts when its time comes, it gets a room or it's concluded", () => {
    expect(matchStarted(match(60), NOW)).toBe(false);
    expect(matchStarted(match(0), NOW)).toBe(true);
    expect(matchStarted(match(null), NOW)).toBe(false);
    expect(matchStarted(match(60, { rooms: [{ mp_id: 1, url: "" }] }), NOW)).toBe(true);
    expect(matchStarted(match(60, { concluded_at: at(-5) }), NOW)).toBe(true);
  });

  it("can be predicted in an open stage with a best of, both teams and time left", () => {
    const open = { status: "open", mode: "auto", best_of: 9 } as const;
    expect(canPredictMatch(open, match(60), NOW)).toBe(true);
    expect(canPredictMatch(open, match(-1), NOW)).toBe(false);
    expect(canPredictMatch(open, match(60, { team2_id: null }), NOW)).toBe(false);
    expect(canPredictMatch({ ...open, best_of: null }, match(60), NOW)).toBe(false);
    expect(canPredictMatch({ ...open, status: "locked" }, match(60), NOW)).toBe(false);
  });

  it("in a stage a host opened, locks on its room or its result, not on the clock", () => {
    const opened = { status: "open", mode: "open", best_of: 9 } as const;
    expect(matchClosed(opened, match(-60), NOW)).toBe(false);
    expect(matchClosed(opened, match(60, { rooms: [{ mp_id: 1, url: "" }] }), NOW)).toBe(true);
    expect(matchClosed(opened, match(-60, { concluded_at: at(-5) }), NOW)).toBe(true);
    expect(canPredictMatch(opened, match(-60), NOW)).toBe(true);
    expect(isMatchLocked(opened, match(-60), NOW)).toBe(false);
    expect(isMatchLocked(opened, match(-60, { rooms: [{ mp_id: 1, url: "" }] }), NOW)).toBe(true);
  });

  it("shows its split once the stage locks, or once it's locked in a stage a host opened", () => {
    const stage = (status: string, mode = "auto") =>
      ({ status, mode }) as Parameters<typeof isMatchLocked>[0];
    expect(isMatchLocked(stage("locked"), match(60), NOW)).toBe(true);
    expect(isMatchLocked(stage("finished"), match(60), NOW)).toBe(true);
    expect(isMatchLocked(stage("open"), match(60), NOW)).toBe(false);
    expect(isMatchLocked(stage("open"), match(-1), NOW)).toBe(true);
    expect(isMatchLocked(stage("upcoming"), match(-1), NOW)).toBe(false);
  });
});

describe("defaultStage", () => {
  const stage = (id: string, status: string) =>
    ({ id, status }) as Parameters<typeof defaultStage>[0][number];

  it("opens on the open stage, else the last that locked, else the first", () => {
    expect(
      defaultStage([stage("a", "finished"), stage("b", "open"), stage("c", "upcoming")])?.id,
    ).toBe("b");
    expect(
      defaultStage([stage("a", "finished"), stage("b", "locked"), stage("c", "upcoming")])?.id,
    ).toBe("b");
    expect(defaultStage([stage("a", "upcoming"), stage("b", "upcoming")])?.id).toBe("a");
    expect(defaultStage([])).toBeUndefined();
  });
});
