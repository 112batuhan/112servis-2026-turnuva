import { describe, expect, it } from "vitest";
import type { MapPick, PublicPoolMap } from "../api/types";
import type { PoolEntry } from "./matchPool";
import { clock, draftRefusal, parseBeatmapId, shownEnd } from "./playerRoom";

const entry = (id: string, abbreviation: string, extra: Partial<PoolEntry> = {}): PoolEntry => ({
  label: `${abbreviation}1`,
  abbreviation,
  color: "#ffffff",
  freemod: false,
  tiebreaker: false,
  repeatable: abbreviation === "NM",
  map: { id } as PublicPoolMap,
  ...extra,
});

describe("draftRefusal mirrors the bot's refusal (autoref/draft.rs)", () => {
  const nm1 = entry("nm1", "NM");
  const nm2 = entry("nm2", "NM");
  const hd1 = entry("hd1", "HD");
  const hd2 = entry("hd2", "HD");
  const tb = entry("tb", "TB", { tiebreaker: true });
  const entries = [nm1, nm2, hd1, hd2, tb];
  const pick = (map_id: string, team_id: string, created_at: string): MapPick => ({
    map_id,
    team_id,
    created_at,
  });

  it("refuses the tiebreaker, banned and played maps", () => {
    const draft = { bans: [{ map_id: "nm1", team_id: "a", created_at: "1" }], picks: [] };
    expect(draftRefusal(tb, "ban", "a", draft, new Set(), entries)).toMatch(/Tiebreaker/);
    expect(draftRefusal(nm1, "pick", "a", draft, new Set(), entries)).toBe("Banlandı");
    expect(draftRefusal(nm2, "ban", "a", draft, new Set(["nm2"]), entries)).toBe("Oynandı");
    expect(draftRefusal(hd1, "ban", "a", draft, new Set(), entries)).toBeNull();
  });

  it("refuses the same mod twice in a row, NoMod aside, by the team's own last pick", () => {
    const draft = {
      bans: [],
      picks: [pick("nm1", "a", "1"), pick("hd1", "a", "3"), pick("nm2", "b", "4")],
    };
    expect(draftRefusal(hd2, "pick", "a", draft, new Set(), entries)).toMatch(/aynı mod/);
    expect(draftRefusal(hd2, "pick", "b", draft, new Set(), entries)).toBeNull();
    expect(draftRefusal(hd2, "ban", "a", draft, new Set(), entries)).toBeNull();
    const nm = { bans: [], picks: [pick("nm1", "a", "1")] };
    expect(draftRefusal(nm2, "pick", "a", nm, new Set(), entries)).toBeNull();
  });
});

describe("parseBeatmapId", () => {
  it("reads ids and osu! beatmap links", () => {
    expect(parseBeatmapId(" 129891 ")).toBe(129891);
    expect(parseBeatmapId("https://osu.ppy.sh/beatmapsets/39804#osu/129891")).toBe(129891);
    expect(parseBeatmapId("https://osu.ppy.sh/beatmaps/129891?mode=osu")).toBe(129891);
    expect(parseBeatmapId("osu.ppy.sh/b/129891")).toBe(129891);
  });

  it("names no beatmap for a set or anything else", () => {
    expect(parseBeatmapId("https://osu.ppy.sh/beatmapsets/39804")).toBeNull();
    expect(parseBeatmapId("hd1")).toBeNull();
    expect(parseBeatmapId("")).toBeNull();
  });
});

describe("the clock", () => {
  it("shows the end the room was told", () => {
    const deadline = "2026-01-01T12:00:10Z";
    expect(shownEnd(deadline, null)).toBe(new Date("2026-01-01T12:00:00Z").getTime());
    // The match's time has no grace.
    expect(shownEnd(deadline, deadline)).toBe(new Date(deadline).getTime());
    expect(shownEnd(null, null)).toBeNull();
  });

  it("reads minutes and seconds, never below zero", () => {
    expect(clock(125_000)).toBe("2:05");
    expect(clock(8_200)).toBe("0:09");
    expect(clock(-5)).toBe("0:00");
  });
});
