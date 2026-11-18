import type { Category, CategoryKind, PublicPoolMap } from "../api/types";

// What a pool category is and the rules its kind brings (backend/src/category.rs), and the
// labels its slots go by. A category's short name labels its slots ("NM1", "TB"): what players
// type in the room to ban and pick.

/** The kinds in the order the editor offers them, with what each means for the rules. */
export const CATEGORY_KINDS: { kind: CategoryKind; label: string; hint: string }[] = [
  { kind: "nomod", label: "NoMod", hint: "Bir takım arka arkaya iki kez seçebilir" },
  { kind: "modded", label: "Modlu", hint: "HD, HR, DT…: aynı takım arka arkaya seçemez" },
  { kind: "freemod", label: "Freemod", hint: "Oyuncular modunu seçer, mapler HR değerleriyle" },
  {
    kind: "tiebreaker",
    label: "Tiebreaker",
    hint: "Banlanmaz, seçilmez; skor eşitlenince oynanır",
  },
];

/** A map added as FM or TB rather than with mods: played with Freemod, stats without mods. */
export const FREEMOD_MAP = "FM";
export const TIEBREAKER_MAP = "TB";

export function isKindMark(mods: string): boolean {
  const m = mods.trim().toUpperCase();
  return m === FREEMOD_MAP || m === TIEBREAKER_MAP;
}

/** The players choose their own mods on it: its category is FM or TB, or it was added as one. */
export function playsFreemod(map: Pick<PublicPoolMap, "mods">, kind: CategoryKind): boolean {
  return kind === "freemod" || kind === "tiebreaker" || isKindMark(map.mods);
}

export function isTiebreaker(map: Pick<PublicPoolMap, "mods">, kind: CategoryKind): boolean {
  return kind === "tiebreaker" || map.mods.trim().toUpperCase() === TIEBREAKER_MAP;
}

/**
 * Whether a map's card adds its HR stats: an FM map, where players may take HR (the tiebreaker is
 * freemod too but doesn't need them), or a map without mods in a freemod category. A map in the
 * shared pool has no category, so only its own FM mark counts there.
 */
export function showsHrStats(map: Pick<PublicPoolMap, "mods">, kind?: CategoryKind): boolean {
  const mods = map.mods.trim().toUpperCase();
  return mods === FREEMOD_MAP || (mods === "" && kind === "freemod");
}

/** A slot's label: the short name and its place ("HD2"), or just the short name when it's alone. */
export function slotLabel(category: Pick<Category, "abbreviation">, index: number, slots: number) {
  return slots > 1 ? `${category.abbreviation}${index + 1}` : category.abbreviation;
}

/** A short name as it's saved: upper case, 1-6 letters (the server checks it again). */
export function cleanAbbreviation(typed: string): string {
  return typed.trim().toLocaleUpperCase("en-US");
}

export const isAbbreviation = (abbr: string) => /^[A-Z]{1,6}$/.test(abbr);

/** The kind a short name usually means, to preselect it while the editor types: NM, FM, TB. */
export function kindOf(abbr: string): CategoryKind | null {
  switch (cleanAbbreviation(abbr)) {
    case "NM":
      return "nomod";
    case "FM":
      return "freemod";
    case "TB":
      return "tiebreaker";
    default:
      return null;
  }
}
