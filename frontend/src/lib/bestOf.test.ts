import { describe, expect, it } from "vitest";
import { firstTo, raceOf, stageLabel } from "./bestOf";

// raceOf mirrors race_of in backend/src/bancho/tally.rs, which the room's bot goes by.

describe("firstTo", () => {
  it("is the majority of the best of", () => {
    expect([1, 3, 9, 11, 13].map(firstTo)).toEqual([1, 2, 5, 6, 7]);
  });
});

describe("raceOf", () => {
  it("reads the same scores as the server", () => {
    expect(raceOf(9, 4, 4)).toEqual({ kind: "tiebreaker" });
    expect(raceOf(9, 5, 3)).toEqual({ kind: "won", slot: 1 });
    expect(raceOf(9, 3, 5)).toEqual({ kind: "won", slot: 2 });
    expect(raceOf(9, 4, 2)).toEqual({ kind: "matchPoint", slots: [1] });
    expect(raceOf(9, 6, 0)).toEqual({ kind: "over" });
    expect(raceOf(9, 5, 5)).toEqual({ kind: "over" });
    expect(raceOf(9, 2, 3)).toEqual({ kind: "open" });
  });

  it("has no tiebreaker in a best of 1", () => {
    expect(raceOf(1, 0, 0)).toEqual({ kind: "matchPoint", slots: [1, 2] });
    expect(raceOf(1, 1, 0)).toEqual({ kind: "won", slot: 1 });
  });
});

describe("stageLabel", () => {
  it("adds the best of when the stage has one", () => {
    expect(stageLabel({ name: "Quarterfinals", best_of: 9 })).toBe("Quarterfinals · Bo9");
    expect(stageLabel({ name: "Qualifiers", best_of: null })).toBe("Qualifiers");
  });
});
