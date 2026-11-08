import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { fetchPublicBracket } from "../../api/bracket";
import { fetchPublicStages } from "../../api/mappool";
import { fetchStageStats } from "../../api/stats";
import type { BracketMatch, PublicBracket } from "../../api/types";
import { useLiveRefresh } from "../../lib/useLiveRefresh";
import { useLiveBracket } from "../../live/LiveContext";

// What the overlay's pages show beyond the match on stream: the whole bracket (the schedule, the
// bracket, the teams' histories, where the winner goes next), kept current like /bracket, and each
// team's seed from the qualifier.

/** The public bracket, reloaded on every change the live socket reports (polled while it's down). */
export function useOverlayBracket(): PublicBracket | null {
  const [bracket, setBracket] = useState<PublicBracket | null>(null);
  const load = useCallback(() => {
    fetchPublicBracket()
      .then(setBracket)
      .catch(() => {
        // The stream keeps the last bracket it had; the next change tries again.
      });
  }, []);
  useEffect(load, [load]);
  useLiveBracket(load, load);
  useLiveRefresh(bracket?.matches ?? [], load);
  return bracket;
}

// Seeds only change while the qualifier is played, so they're read again now and then.
const SEEDS_REFRESH_MS = 5 * 60_000;

/** Each team's qualifier seed by team id; empty until the qualifier has been counted. */
export function useQualifierSeeds(): ReadonlyMap<string, number> {
  const [seeds, setSeeds] = useState<ReadonlyMap<string, number>>(new Map());
  useEffect(() => {
    let stopped = false;
    const load = async () => {
      try {
        const qualifier = (await fetchPublicStages()).find((s) => s.is_qualifier);
        if (!qualifier) return;
        const stats = await fetchStageStats(qualifier.id);
        if (stopped) return;
        setSeeds(new Map(stats.stats.teams.flatMap((t) => (t.rank === null ? [] : [[t.team_id, t.rank] as const]))));
      } catch {
        // No published qualifier, or not counted yet: the page leaves the seed out.
      }
    };
    load();
    const timer = window.setInterval(load, SEEDS_REFRESH_MS);
    return () => {
      stopped = true;
      window.clearInterval(timer);
    };
  }, []);
  return seeds;
}

/**
 * `value` as it was when `hold` became true, until it's false again: what the overlay shows while
 * the tournament clients play a map, so nothing the site learned early reaches the stream.
 */
export function useHeld<T>(value: T, hold: boolean): T {
  const kept = useRef(value);
  if (!hold) kept.current = value;
  return kept.current;
}

/** The stage the schedule shows: the match on stream's, else the one being played, else the next to play. */
export function currentStageId(matches: BracketMatch[], onStream: BracketMatch | null): string | null {
  if (onStream?.stage) return onStream.stage.id;
  const live = matches.find((m) => m.stage && !m.concluded_at && m.rooms.length > 0);
  if (live?.stage) return live.stage.id;
  const open = matches
    .filter((m) => m.stage && !m.concluded_at)
    .sort((a, b) => (a.stage?.position ?? 0) - (b.stage?.position ?? 0));
  if (open[0]?.stage) return open[0].stage.id;
  const last = [...matches].filter((m) => m.stage).sort((a, b) => (b.stage?.position ?? 0) - (a.stage?.position ?? 0));
  return last[0]?.stage?.id ?? null;
}

/** A team's bracket matches this tournament, newest first, leaving out `except`. */
export function useTeamHistory(bracket: PublicBracket | null, teamId: string | null, except: string | null) {
  return useMemo(() => {
    if (!bracket || !teamId) return [];
    return bracket.matches
      .filter((m) => m.id !== except && m.concluded_at && (m.team1_id === teamId || m.team2_id === teamId))
      .sort((a, b) => (b.concluded_at ?? "").localeCompare(a.concluded_at ?? ""));
  }, [bracket, teamId, except]);
}
