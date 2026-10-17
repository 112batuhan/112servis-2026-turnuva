// How many maps a stage's bracket matches are played to (Stage.best_of): a Bo9 match goes to the
// first team to win 5, and at 4-4 the tiebreaker decides it. The referee page, the public match
// page and every view that names a match's stage read it from here.

/** The choices the hosts get; the server takes any odd number from 1 to 25. */
export const BEST_OF_CHOICES = [1, 3, 5, 7, 9, 11, 13] as const;

/** Map wins a team needs to take the match. */
export function firstTo(bestOf: number): number {
  return Math.ceil(bestOf / 2);
}

/** "Bo9". */
export function bestOfLabel(bestOf: number): string {
  return `Bo${bestOf}`;
}

/** A stage's name as the match views show it, with its best of when set: "Çeyrek Final · Bo9". */
export function stageLabel(stage: { name: string; best_of: number | null }): string {
  return stage.best_of ? `${stage.name} · ${bestOfLabel(stage.best_of)}` : stage.name;
}

/** Where a bracket match stands against its best of. */
export type Race =
  /** Neither team is one map away. */
  | { kind: "open" }
  /** One team (or, before the tiebreaker, neither) is a map away: `slots` are the teams on match point. */
  | { kind: "matchPoint"; slots: (1 | 2)[] }
  /** Both teams are a map away, so the tiebreaker is played. */
  | { kind: "tiebreaker" }
  /** A team has the wins it needs; the referee still concludes it. */
  | { kind: "won"; slot: 1 | 2 }
  /** More maps were counted than the best of allows; something needs a look. */
  | { kind: "over" };

export function raceOf(bestOf: number, team1: number, team2: number): Race {
  const need = firstTo(bestOf);
  if (team1 > need || team2 > need || (team1 >= need && team2 >= need)) return { kind: "over" };
  if (team1 === need) return { kind: "won", slot: 1 };
  if (team2 === need) return { kind: "won", slot: 2 };
  if (need > 1 && team1 === need - 1 && team2 === need - 1) return { kind: "tiebreaker" };
  const slots = ([1, 2] as const).filter((s) => (s === 1 ? team1 : team2) === need - 1);
  return slots.length > 0 ? { kind: "matchPoint", slots } : { kind: "open" };
}
