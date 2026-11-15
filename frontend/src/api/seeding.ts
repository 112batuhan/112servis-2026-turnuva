import { del, get, post, put } from "./client";
import type { BracketFormat, Seeding } from "./types";

// Host-only.
export const fetchSeeding = () => get<Seeding>("/api/seeding");
/** Saves the hosts' order, best seed first. */
export const saveSeeding = (teamIds: string[]) => put<void>("/api/seeding", { team_ids: teamIds });
/** Hands the seeding back to the qualifier's order. */
export const resetSeeding = () => del("/api/seeding");
/** Makes the bracket from the whole order (the format takes its top) with a stage for each week. */
export const makeBracket = (
  format: BracketFormat,
  teamIds: string[],
  stageIds: (string | null)[],
) => post<void>("/api/seeding", { format, team_ids: teamIds, stage_ids: stageIds });
/** Takes the whole bracket down while none of it has been played. */
export const removeBracket = () => del("/api/bracket");
