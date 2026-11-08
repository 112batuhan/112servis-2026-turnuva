import { useEffect, useMemo, useRef, useState } from "react";
import { useMatchPage, type MatchPageState } from "../match/useMatchPage";
import { IPC, type TosuTourney } from "./tosu";

// The match as the overlays show it: the site's data (the same reading the public match page
// follows), held back while the tournament clients play a map, plus the live score of that map
// from tosu.
//
// Why hold: the room's bot hears the map finish on Bancho before the spectator clients, which
// run behind the players, get there; the site then knows who won (the stars, the map's winner,
// even the next pick) while viewers are still watching the map being played. So from the moment
// the clients start a map until every one of them shows the results screen, the overlays keep the
// match exactly as it was when the map started. A tourney client that never leaves "playing"
// (closed mid-map, tosu gone) releases it once the map's length and a margin have passed.

const HOLD_MARGIN_S = 120;
const HOLD_FALLBACK_S = 600;

/** A map's score on stream, team 1 then team 2. */
export type LiveTotals = [number, number];

export interface BroadcastMatch {
  /** The site's match as it's shown: frozen while the clients play a map. */
  page: MatchPageState;
  /** The site's results are being held back right now. */
  holding: boolean;
  /** The map being played (or the last one played), summed per team from tosu; null without tosu. */
  totals: LiveTotals | null;
}

export function useBroadcastMatch(matchId: string, tosu: TosuTourney): BroadcastMatch {
  const latest = useMatchPage(matchId, { spectate: false });
  const playing = tosu.ipcState === IPC.playing;

  // The match as it was when the clients started the map.
  const kept = useRef<MatchPageState>(latest);
  const [expired, setExpired] = useState(false);
  const length = kept.current.currentEntry?.map.total_length;
  const limitMs = ((length ?? HOLD_FALLBACK_S) + HOLD_MARGIN_S) * 1000;
  useEffect(() => {
    setExpired(false);
    if (!playing) return;
    const timer = window.setTimeout(() => setExpired(true), limitMs);
    return () => window.clearTimeout(timer);
  }, [playing, limitMs]);
  const holding = playing && !expired && kept.current.match !== null;
  if (!holding) kept.current = latest;

  // Each team's summed score, matching tosu's clients to the match's players by osu! id (so the
  // side tosu guesses from the client order doesn't matter), falling back to its guess.
  const roster = latest.roster;
  const current = useMemo<LiveTotals | null>(() => {
    if (tosu.clients.length === 0) return null;
    const totals: LiveTotals = [0, 0];
    for (const c of tosu.clients) {
      const player = roster.find((r) => r.player.osu_id === c.userId);
      const side = player?.color ? (player.color === "blue" ? 0 : 1) : c.team === "left" ? 0 : 1;
      totals[side] += c.score;
    }
    return totals;
  }, [tosu.clients, roster]);

  // The results screen may already have zeroed the clients' scores; the last map's totals stay up
  // until the next map starts.
  const shownTotals = useRef<LiveTotals | null>(null);
  if (current === null) shownTotals.current = null;
  else if (playing || current[0] + current[1] > 0) shownTotals.current = current;

  return { page: kept.current, holding, totals: shownTotals.current };
}
