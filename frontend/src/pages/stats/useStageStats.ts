import { useCallback, useEffect, useRef, useState } from "react";
import { errorMessage } from "../../api/client";
import { fetchStageActivity, fetchStageStats } from "../../api/stats";
import type { StageActivity, StageStats } from "../../api/types";
import { useLive } from "../../live/LiveContext";

// A finished map pushes a score (bracket) or a match change (a lobby, osu!'s accuracy):
// a burst of them is gathered into one reload.
const RELOAD_DELAY_MS = 1500;
// The statistics don't need the socket: they're read again this often while the tab is shown.
const POLL_MS = 60_000;

/**
 * A stage's statistics, kept current: read again after the live socket says a score or a match
 * changed, every minute while the tab is visible, and when the tab comes back into view.
 */
export function useStageStats(stageId: string | null) {
  const [data, setData] = useState<StageStats | null>(null);
  const [activity, setActivity] = useState<StageActivity[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loadedAt, setLoadedAt] = useState<Date | null>(null);
  const [loading, setLoading] = useState(false);
  const current = useRef(stageId);
  current.current = stageId;

  const reload = useCallback(async () => {
    const id = current.current;
    fetchStageActivity()
      .then(setActivity)
      .catch(() => {});
    if (!id) return;
    setLoading(true);
    try {
      const next = await fetchStageStats(id);
      // A stage picked while this was on its way wins.
      if (current.current !== id) return;
      setData(next);
      setLoadedAt(new Date());
      setError(null);
    } catch (e) {
      if (current.current === id) setError(errorMessage(e));
    } finally {
      if (current.current === id) setLoading(false);
    }
  }, []);

  useEffect(() => {
    void reload();
  }, [stageId, reload]);

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

  return {
    data: data && data.pool.id === stageId ? data : null,
    activity,
    error,
    clearError: () => setError(null),
    loadedAt,
    loading,
    reload,
  };
}
