import type { BracketMatch, Stage } from "../api/types";
import { compareByTime } from "./bracket";

// Which stage the tournament is in, from its matches. The landing page's track and the mappool
// page (which opens on it and lights it) read it from here, so they always agree.

export type StageState = "done" | "current" | "upcoming";

export interface TrackStage {
  id: string;
  name: string;
  state: StageState;
}

/**
 * The tournament's stages in play order (the qualifier first), each marked done, current or
 * upcoming from its matches: the current one is where the earliest unfinished match is. Bracket
 * stages that aren't in `stages` (an unpublished pool) still come from their matches.
 */
export function stageTrack(
  matches: BracketMatch[],
  lobbies: { concluded_at: string | null }[],
  stages: Stage[],
): TrackStage[] {
  const byId = new Map<string, { id: string; name: string; position: number; qualifier: boolean }>();
  for (const s of stages) byId.set(s.id, { id: s.id, name: s.name, position: s.position, qualifier: s.is_qualifier });
  for (const m of matches) {
    if (m.stage && !byId.has(m.stage.id))
      byId.set(m.stage.id, { id: m.stage.id, name: m.stage.name, position: m.stage.position, qualifier: false });
  }
  const ordered = [...byId.values()].sort((a, b) => Number(b.qualifier) - Number(a.qualifier) || a.position - b.position);

  const qualifiersOpen = lobbies.some((l) => !l.concluded_at);
  const nextMatch = matches.filter((m) => !m.concluded_at).sort(compareByTime)[0];
  // Once a bracket match is played the qualifiers are over, even if a lobby was never closed.
  const bracketStarted = matches.some((m) => m.concluded_at);
  const currentId = qualifiersOpen && !bracketStarted
    ? ordered.find((s) => s.qualifier)?.id
    : nextMatch?.stage?.id;

  const currentIndex = ordered.findIndex((s) => s.id === currentId);
  return ordered.map((s, i) => {
    const own = matches.filter((m) => m.stage?.id === s.id);
    const finished = s.qualifier
      ? (lobbies.length > 0 && !qualifiersOpen) || bracketStarted
      : own.length > 0 && own.every((m) => m.concluded_at);
    const state: StageState =
      s.id === currentId ? "current" : finished || (currentIndex >= 0 && i < currentIndex) ? "done" : "upcoming";
    return { id: s.id, name: s.name, state };
  });
}

/** The stage the tournament is in, or null before anything is scheduled. */
export function currentStageId(
  matches: BracketMatch[],
  lobbies: { concluded_at: string | null }[],
  stages: Stage[],
): string | null {
  return stageTrack(matches, lobbies, stages).find((s) => s.state === "current")?.id ?? null;
}
