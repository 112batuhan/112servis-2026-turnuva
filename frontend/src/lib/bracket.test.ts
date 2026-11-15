import { describe, expect, it } from "vitest";
import type { BracketMatch } from "../api/types";
import { BOARD, currentStageId, isLive, overlapsAny, potentialRows, scoreHalves, winningSlot, withPotentials } from "./bracket";

// The bracket rules the views mirror from the server: who won (services/bracket.rs `outcome`), the stage being
// played (stage_track.rs),
// when a match is live, potential matches as rows, and the board's no-overlap rule (frontend only).

const NOW = Date.parse("2026-11-10T12:00:00Z");
const at = (minutes: number) => new Date(NOW + minutes * 60_000).toISOString();

function match(over: Partial<BracketMatch> = {}): BracketMatch {
  return {
    id: "m",
    side: "winners",
    number: 1,
    pos_x: 0,
    pos_y: 0,
    starts_at: null,
    team1_id: "t1",
    team2_id: "t2",
    team1_score: null,
    team2_score: null,
    forfeit_slot: null,
    winner_to: null,
    loser_to: null,
    stage: null,
    referee: null,
    streamer: null,
    rooms: [],
    concluded_at: null,
    potentials: [],
    ...over,
  };
}

const room = { mp_id: 1, url: "https://osu.ppy.sh/community/matches/1" };

describe("winningSlot", () => {
  it("is the higher score once concluded", () => {
    expect(winningSlot(match({ team1_score: 5, team2_score: 3, concluded_at: at(0) }))).toBe(1);
    expect(winningSlot(match({ team1_score: 2, team2_score: 5, concluded_at: at(0) }))).toBe(2);
  });

  it("is nobody before the match is concluded, or on a level or missing score", () => {
    expect(winningSlot(match({ team1_score: 5, team2_score: 3 }))).toBeNull();
    expect(winningSlot(match({ team1_score: 4, team2_score: 4, concluded_at: at(0) }))).toBeNull();
    expect(winningSlot(match({ concluded_at: at(0) }))).toBeNull();
  });

  it("is the other team after a forfeit, whatever the score", () => {
    expect(winningSlot(match({ forfeit_slot: 1, team1_score: 5, team2_score: 0, concluded_at: at(0) }))).toBe(2);
    expect(winningSlot(match({ forfeit_slot: 2, concluded_at: at(0) }))).toBe(1);
  });
});

describe("scoreHalves", () => {
  it("shows W and FF for a forfeit, else the figures", () => {
    expect(scoreHalves(match({ forfeit_slot: 1, concluded_at: at(0) }))).toEqual(["FF", "W"]);
    expect(scoreHalves(match({ team1_score: 3, team2_score: 1 }))).toEqual(["3", "1"]);
    expect(scoreHalves(match())).toBeNull();
  });
});

describe("isLive", () => {
  it("needs its time to have come and a room, until it's concluded", () => {
    expect(isLive(match({ starts_at: at(-5), rooms: [room] }), NOW)).toBe(true);
    expect(isLive(match({ starts_at: at(5), rooms: [room] }), NOW)).toBe(false);
    expect(isLive(match({ starts_at: at(-5) }), NOW)).toBe(false);
    expect(isLive(match({ rooms: [room] }), NOW)).toBe(false);
    expect(isLive(match({ starts_at: at(-5), rooms: [room], concluded_at: at(0) }), NOW)).toBe(false);
  });
});

describe("potential matches", () => {
  const referee = { id: "r", username: "Ref", avatar_url: "" } as unknown as BracketMatch["referee"];
  const undecided = match({
    id: "L20",
    team1_id: "t1",
    team2_id: null,
    team1_score: 1,
    rooms: [room],
    potentials: [
      { id: "p1", team1_id: "t1", team2_id: "t3", starts_at: at(60), referee, streamer: null },
      { id: "p2", team1_id: "t1", team2_id: "t4", starts_at: null, referee: null, streamer: null },
    ],
  });

  it("stand in for their match with their own teams, time and staff, pointing back at it", () => {
    const [first, second] = potentialRows(undecided);
    expect(first).toMatchObject({ id: "p1", team2_id: "t3", starts_at: at(60), referee, potential_of: "L20" });
    expect(second).toMatchObject({ id: "p2", team2_id: "t4", starts_at: null, potential_of: "L20" });
    // No room, score or pairings of their own.
    expect(first.rooms).toEqual([]);
    expect(first.team1_score).toBeNull();
    expect(first.potentials).toEqual([]);
  });

  it("replace only the undecided matches", () => {
    const decided = match({ id: "W1" });
    expect(withPotentials([decided, undecided]).map((m) => m.id)).toEqual(["W1", "p1", "p2"]);
  });
});

describe("overlapsAny", () => {
  const card = match({ id: "a", pos_x: 0, pos_y: 0 });

  it("refuses a spot any card's rectangle covers", () => {
    expect(overlapsAny([card], { x: BOARD.cardW - 1, y: BOARD.cardH - 1 })).toBe(true);
    expect(overlapsAny([card], { x: -(BOARD.cardW - 1), y: 0 })).toBe(true);
  });

  it("allows cards edge to edge", () => {
    expect(overlapsAny([card], { x: BOARD.cardW, y: 0 })).toBe(false);
    expect(overlapsAny([card], { x: 0, y: -BOARD.cardH })).toBe(false);
  });

  it("ignores the card being moved", () => {
    expect(overlapsAny([card], { x: 1, y: 1 }, "a")).toBe(false);
  });
});

describe("currentStageId", () => {
  const stage = (id: string, position: number) => ({ id, name: id, abbreviation: null, published: true, position, best_of: 7 });
  const r16 = stage("r16", 1);
  const qf = stage("qf", 2);

  it("is the stage of the earliest unfinished match, unscheduled ones last", () => {
    const matches = [
      match({ id: "a", stage: r16, starts_at: at(-60), concluded_at: at(-10) }),
      match({ id: "b", stage: qf, starts_at: null }),
      match({ id: "c", stage: qf, starts_at: at(60) }),
      match({ id: "d", stage: r16, starts_at: at(30) }),
    ];
    expect(currentStageId(matches)).toBe("r16");
    expect(currentStageId(matches.filter((m) => m.id !== "d"))).toBe("qf");
  });

  it("breaks a tie on time by code, winners first", () => {
    const matches = [
      match({ id: "l", side: "losers", number: 1, stage: qf, starts_at: at(0) }),
      match({ id: "w", side: "winners", number: 2, stage: r16, starts_at: at(0) }),
    ];
    expect(currentStageId(matches)).toBe("r16");
  });

  it("skips matches without a stage, and is null once every staged match is over", () => {
    expect(currentStageId([match({ starts_at: at(0) }), match({ id: "b", stage: qf, starts_at: at(10) })])).toBe("qf");
    expect(currentStageId([match({ stage: qf, concluded_at: at(0) }), match({ id: "b" })])).toBeNull();
  });
});
