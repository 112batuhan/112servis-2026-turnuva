import { useCallback, useEffect, useMemo, useState } from "react";
import { fetchPublicBracket } from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import { fetchPickem, predictMatch, withdrawPrediction } from "../../../api/pickem";
import type {
  BracketMatch,
  MyPrediction,
  PickemOverview,
  PublicBracket,
  Team,
} from "../../../api/types";
import { firstTo } from "../../../lib/bestOf";
import { withScore } from "../../../lib/bracket";
import { useLiveRefresh } from "../../../lib/useLiveRefresh";
import { useLiveBracket } from "../../../live/LiveContext";

/**
 * The pick'em page's data: the stages' pick'em with the viewer's predictions and the standings,
 * plus the bracket for the matches and teams. Follows the bracket live, so results and scores land
 * as they're saved. A prediction shows at once and saves in the background.
 */
export function usePickem() {
  const [overview, setOverview] = useState<PickemOverview | null>(null);
  const [bracket, setBracket] = useState<PublicBracket | null>(null);
  const [error, setError] = useState<string | null>(null);

  const reload = useCallback(async () => {
    const [o, b] = await Promise.all([fetchPickem(), fetchPublicBracket()]);
    setOverview(o);
    setBracket(b);
  }, []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  const matches = useMemo(() => bracket?.matches ?? [], [bracket]);
  // A failed refresh keeps what's shown; the next one tries again.
  useLiveRefresh(matches, () => reload().catch(() => {}));
  useLiveBracket(
    (e) =>
      setBracket(
        (b) =>
          b && { ...b, matches: withScore(b.matches, e.match_id, e.team1_score, e.team2_score) },
      ),
    () => reload().catch(() => {}),
  );

  const teams = useMemo(
    () => new Map<string, Team>(bracket?.teams.map((t) => [t.id, t])),
    [bracket],
  );
  const mine = useMemo(() => new Map(overview?.mine.map((p) => [p.match_id, p])), [overview]);
  const splits = useMemo(() => new Map(overview?.splits.map((s) => [s.match_id, s])), [overview]);

  const setMine = (matchId: string, next: MyPrediction | null) =>
    setOverview(
      (o) =>
        o && {
          ...o,
          mine: [...o.mine.filter((p) => p.match_id !== matchId), ...(next ? [next] : [])],
        },
    );

  /** Saves a prediction; on failure the old one comes back and the error is thrown to the row. */
  async function predict(match: BracketMatch, teamId: string, loserScore: number) {
    const before = mine.get(match.id) ?? null;
    const bestOf = match.stage?.best_of ?? 1;
    setMine(match.id, {
      match_id: match.id,
      team_id: teamId,
      winner_score: firstTo(bestOf),
      loser_score: loserScore,
      points: null,
      void: false,
    });
    try {
      await predictMatch(match.id, teamId, loserScore);
    } catch (e) {
      setMine(match.id, before);
      throw e;
    }
    // The stage's pickers count may have moved.
    fetchPickem().then(setOverview, () => {});
  }

  async function withdraw(match: BracketMatch) {
    const before = mine.get(match.id) ?? null;
    setMine(match.id, null);
    try {
      await withdrawPrediction(match.id);
    } catch (e) {
      setMine(match.id, before);
      throw e;
    }
    fetchPickem().then(setOverview, () => {});
  }

  return {
    overview,
    bracket,
    matches,
    teams,
    mine,
    splits,
    error,
    clearError: () => setError(null),
    reload: () => reload().catch((e) => setError(errorMessage(e))),
    predict,
    withdraw,
  };
}
