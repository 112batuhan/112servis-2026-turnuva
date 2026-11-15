import type { BracketFormat, Stage } from "../api/types";

/** How many teams each format seeds; the rest are out of the tournament. */
export const FORMAT_TEAMS: Record<BracketFormat, number> = { round_of_32: 32, round_of_16: 16 };

export const FORMAT_LABELS: Record<BracketFormat, string> = {
  round_of_32: "Round of 32",
  round_of_16: "Round of 16",
};

export interface BracketWeek {
  /** The rounds played that week: "W1", "W2 · L1", "Winners final · L6–L7". */
  rounds: string;
  matches: number;
  /** The stage name the dev seed and most tournaments give that week. */
  usual: string;
}

const USUAL = ["Round of 32", "Round of 16", "Quarterfinals", "Semifinals", "Finals", "Grand Final"];

/**
 * The weeks a format is played over, as the backend lays the bracket out (`seeding.rs`): winners
 * round r in week r - 1, losers round k in week ⌊k/2⌋ + 1, and the losers final, the grand final and
 * its reset in the last week.
 */
export function bracketWeeks(format: BracketFormat): BracketWeek[] {
  const teams = FORMAT_TEAMS[format];
  const winnersRounds = Math.log2(teams);
  const losersRounds = 2 * (winnersRounds - 1);
  const usual = USUAL.slice(USUAL.length - winnersRounds - 1);
  return usual.map((name, week) => {
    const parts: string[] = [];
    let matches = 0;
    if (week < winnersRounds) {
      parts.push(week === winnersRounds - 1 ? "Winners final" : `W${week + 1}`);
      matches += teams >> (week + 1);
    } else {
      parts.push("Grand final + reset");
      matches += 2;
    }
    const losers: number[] = [];
    for (let k = 1; k <= losersRounds; k++) {
      if (Math.floor(k / 2) + 1 !== week) continue;
      losers.push(k);
      matches += (teams / 4) >> Math.floor((k - 1) / 2);
    }
    if (losers.length > 0) {
      const last = losers[losers.length - 1];
      const name = last === losersRounds ? "Losers final" : `L${losers.join("–")}`;
      parts.push(losers.length > 1 && last === losersRounds ? `L${losers[0]} · Losers final` : name);
    }
    return { rounds: parts.join(" · "), matches, usual: name };
  });
}

/**
 * Each week's stage to start from: the bracket stage named as that week usually is, else the stage
 * at the same place from the end of the play order (the last week is the grand final's), else none.
 */
export function defaultStages(format: BracketFormat, stages: Stage[]): (string | null)[] {
  const weeks = bracketWeeks(format);
  const offset = stages.length - weeks.length;
  const taken = new Set<string>();
  const byName = weeks.map((w) => {
    const found = stages.find((s) => s.name.trim().toLowerCase() === w.usual.toLowerCase());
    if (found) taken.add(found.id);
    return found?.id ?? null;
  });
  return byName.map((id, i) => {
    if (id) return id;
    const fallback = stages[offset + i];
    if (!fallback || taken.has(fallback.id)) return null;
    taken.add(fallback.id);
    return fallback.id;
  });
}
