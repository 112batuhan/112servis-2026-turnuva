import { useCallback, useEffect, useState } from "react";
import { fetchStageActivity, fetchStageStats } from "../../../api/stats";
import type { StageActivity } from "../../../api/types";
import { createStageCache, useStageCache } from "../../../lib/useStageCache";
import { useLive } from "../../../live/LiveContext";

// A finished map pushes a score (bracket) or a match change (a lobby, osu!'s accuracy):
// a burst of them is gathered into one reload.
const RELOAD_DELAY_MS = 1500;
// The statistics don't need the socket: they're read again this often while the tab is shown.
const POLL_MS = 60_000;

/** Every stage's statistics as last read: a stage opened again shows them at once while they're read again. */
export const statsCache = createStageCache(fetchStageStats);

/**
 * A stage's statistics, kept current: read again after the live socket says a score or a match
 * changed, every minute while the tab is visible, and when the tab comes back into view. While
 * another stage's are on their way the last ones shown stay (`shownId`, `stale`: lib/useStageCache.ts).
 */
export function useStageStats(stageId: string | null) {
  const stage = useStageCache(statsCache, stageId);
  const [activity, setActivity] = useState<StageActivity[] | null>(null);
  const reloadStage = stage.reload;

  const loadActivity = useCallback(() => {
    fetchStageActivity()
      .then(setActivity)
      .catch(() => {});
  }, []);

  const reload = useCallback(async () => {
    loadActivity();
    await reloadStage();
  }, [loadActivity, reloadStage]);

  useEffect(loadActivity, [loadActivity]);

  const { subscribe } = useLive();
  useEffect(() => {
    let timer: number | undefined;
    const off = subscribe((event) => {
      if (event.type !== "score" && event.type !== "bracket" && event.type !== "match") return;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => void reload(), RELOAD_DELAY_MS);
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, [subscribe, reload]);

  useEffect(() => {
    const poll = window.setInterval(() => {
      if (document.visibilityState === "visible") void reload();
    }, POLL_MS);
    const onVisible = () => {
      if (document.visibilityState === "visible") void reload();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      window.clearInterval(poll);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [reload]);

  return { ...stage, activity, reload };
}
