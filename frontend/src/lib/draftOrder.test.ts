import { describe, expect, it } from "vitest";
import {
  DEFAULT_DRAFT_ORDER,
  draftOf,
  formatDraftOrder,
  parseDraftOrder,
  stageDraftOrder,
  type DraftOrder,
} from "./draftOrder";

// The same cases as backend/src/draft.rs, whose rules this file mirrors: if one side changes, the
// other's tests say so.

const A = "team-a";
const B = "team-b";

function order(text: string): DraftOrder {
  const parsed = parseDraftOrder(text);
  if ("error" in parsed) throw new Error(parsed.error);
  return parsed.order;
}

/** The next ban or pick after the teams in `done` marked theirs, in that order. */
function next(o: DraftOrder, done: string[], chosen: string | null = null) {
  const marks = done.map((team_id, i) => ({
    kind: o.steps[i]?.kind ?? "pick",
    map_id: `map-${i}`,
    team_id,
    created_at: new Date(i * 1000).toISOString(),
  }));
  return draftOf(
    o,
    marks.filter((m) => m.kind === "ban"),
    marks.filter((m) => m.kind === "pick"),
    [A, B],
    chosen,
    true,
  ).next;
}

describe("parseDraftOrder", () => {
  it("parses and formats", () => {
    const o = order(" P1 - p2-b2-B1-p1-* ");
    expect(o.steps).toHaveLength(5);
    expect(o.alternate).toBe(true);
    expect(formatDraftOrder(o)).toBe("p1-p2-b2-b1-p1-*");
    expect(formatDraftOrder(order("b1-b2"))).toBe("b1-b2");
  });

  it.each(["", "*", "p1-*-p2", "p3", "x1", "p1--p2", "p12", "p1-"])("refuses %j", (bad) => {
    expect(parseDraftOrder(bad)).toHaveProperty("error");
  });

  it("falls back to the default for a stage without one", () => {
    expect(formatDraftOrder(stageDraftOrder(null))).toBe(DEFAULT_DRAFT_ORDER);
    expect(formatDraftOrder(stageDraftOrder("nonsense"))).toBe(DEFAULT_DRAFT_ORDER);
  });
});

describe("draftOf", () => {
  const o = order("p1-p2-b2-b1-p1-*");

  it("doesn't know who 1 is before the first mark", () => {
    expect(next(o, [])).toEqual({ kind: "pick", teamId: null });
  });

  it("follows the order from whoever marked first", () => {
    // B won the roll and picked first: it's 1.
    expect(next(o, [B])).toEqual({ kind: "pick", teamId: A });
    expect(next(o, [B, A])).toEqual({ kind: "ban", teamId: A });
    expect(next(o, [B, A, A])).toEqual({ kind: "ban", teamId: B });
    expect(next(o, [B, A, A, B])).toEqual({ kind: "pick", teamId: B });
  });

  it("has picks take turns after the listed steps, following who actually picked", () => {
    expect(next(o, [B, A, A, B, B])).toEqual({ kind: "pick", teamId: A });
    expect(next(o, [B, A, A, B, B, B])).toEqual({ kind: "pick", teamId: A });
  });

  it("lets the referee's say stand in for the order", () => {
    expect(next(o, [B, A, A, B, B, B], B)).toEqual({ kind: "pick", teamId: B });
  });

  it("runs out without a closing *", () => {
    const short = order("b2-p1");
    expect(next(short, [A])).toEqual({ kind: "pick", teamId: B });
    expect(next(short, [A, B])).toBeNull();
  });

  it("plays the default as two bans, then picks from the team that didn't ban first", () => {
    expect(next(stageDraftOrder(null), [A, B])).toEqual({ kind: "pick", teamId: B });
  });

  it("lists only what was made once the draft is closed", () => {
    const closed = draftOf(o, [], [{ map_id: "m", team_id: B, created_at: new Date(0).toISOString() }], [A, B], null, false);
    expect(closed.next).toBeNull();
    expect(closed.steps.map((s) => s.state)).toEqual(["done"]);
  });

  it("marks a ban made where the order has a pick", () => {
    const off = draftOf(o, [{ map_id: "m", team_id: B, created_at: new Date(0).toISOString() }], [], [A, B], null, true);
    expect(off.steps[0]).toMatchObject({ kind: "ban", offOrder: true, state: "done" });
  });
});
