import { describe, expect, it } from "vitest";
import {
  cleanAbbreviation,
  isAbbreviation,
  isTiebreaker,
  kindOf,
  playsFreemod,
  showsHrStats,
  slotLabel,
} from "./category";

// Mirrors backend/src/category.rs and the labels of bancho/tally/pool.rs.
describe("category rules", () => {
  it("labels slots with the short name", () => {
    expect(slotLabel({ abbreviation: "HD" }, 1, 2)).toBe("HD2");
    expect(slotLabel({ abbreviation: "TB" }, 0, 1)).toBe("TB");
  });

  it("plays FM and TB categories, and FM or TB maps, with Freemod", () => {
    expect(playsFreemod({ mods: "" }, "freemod")).toBe(true);
    expect(playsFreemod({ mods: "" }, "tiebreaker")).toBe(true);
    expect(playsFreemod({ mods: "FM" }, "modded")).toBe(true);
    expect(playsFreemod({ mods: "HD" }, "modded")).toBe(false);
    expect(isTiebreaker({ mods: "TB" }, "modded")).toBe(true);
    expect(isTiebreaker({ mods: "" }, "freemod")).toBe(false);
  });

  it("shows HR stats on FM maps only", () => {
    expect(showsHrStats({ mods: "FM" }, "freemod")).toBe(true);
    expect(showsHrStats({ mods: "" }, "freemod")).toBe(true);
    expect(showsHrStats({ mods: "TB" }, "tiebreaker")).toBe(false);
    expect(showsHrStats({ mods: "HD" }, "freemod")).toBe(false);
    // The shared pool: no category, so only the map's own FM mark.
    expect(showsHrStats({ mods: "FM" })).toBe(true);
    expect(showsHrStats({ mods: "" })).toBe(false);
    expect(showsHrStats({ mods: "TB" })).toBe(false);
  });

  it("reads short names as letters and guesses their kind", () => {
    expect(cleanAbbreviation(" nm ")).toBe("NM");
    expect(isAbbreviation("HDHR")).toBe(true);
    expect(isAbbreviation("NM1")).toBe(false);
    expect(isAbbreviation("TOOLONG")).toBe(false);
    expect(kindOf("tb")).toBe("tiebreaker");
    expect(kindOf("HD")).toBe(null);
  });
});
