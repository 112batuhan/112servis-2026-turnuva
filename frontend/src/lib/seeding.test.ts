import { describe, expect, it } from "vitest";
import type { Stage } from "../api/types";
import { bracketWeeks, defaultStages } from "./seeding";

const stage = (name: string, position: number): Stage => ({
  id: name,
  name,
  abbreviation: null,
  position,
  published: true,
  is_qualifier: false,
  best_of: null,
  draft_order: null,
  pack_url: null,
  created_at: "",
  updated_at: "",
});

describe("bracketWeeks", () => {
  it("counts every match of a Round of 32, as the backend makes it", () => {
    const weeks = bracketWeeks("round_of_32");
    expect(weeks.map((w) => w.rounds)).toEqual([
      "W1",
      "W2 · L1",
      "W3 · L2–3",
      "W4 · L4–5",
      "Winners final · L6–7",
      "Grand final + reset · Losers final",
    ]);
    expect(weeks.map((w) => w.matches)).toEqual([16, 16, 16, 8, 4, 3]);
    expect(weeks.reduce((n, w) => n + w.matches, 0)).toBe(63);
  });

  it("starts a Round of 16 at the Round of 16", () => {
    const weeks = bracketWeeks("round_of_16");
    expect(weeks.map((w) => w.usual)).toEqual(["Round of 16", "Quarterfinals", "Semifinals", "Finals", "Grand Final"]);
    expect(weeks.reduce((n, w) => n + w.matches, 0)).toBe(31);
  });
});

describe("defaultStages", () => {
  it("takes the stages by name, then by place from the end", () => {
    const stages = ["Round of 32", "Round of 16", "Çeyrek", "Yarı", "Finals", "Grand Final"].map(stage);
    expect(defaultStages("round_of_16", stages)).toEqual(["Round of 16", "Çeyrek", "Yarı", "Finals", "Grand Final"]);
    expect(defaultStages("round_of_32", stages.slice(2))).toEqual([null, null, "Çeyrek", "Yarı", "Finals", "Grand Final"]);
  });
});
