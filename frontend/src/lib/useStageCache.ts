import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { errorMessage } from "../api/client";

/** One stage's data as it was read, and when. */
export interface Loaded<T> {
  id: string;
  data: T;
  loadedAt: Date;
}

/**
 * Each stage's data as last read, kept for as long as the app is open, so going back to a stage
 * (or to the page) shows it at once while it's read again. One per kind of data, at module level.
 */
export interface StageCache<T> {
  get(id: string): Loaded<T> | undefined;
  /** Reads the stage. Unless `fresh`, a request already on its way or a reading this recent is used instead. */
  load(id: string, fresh?: boolean): Promise<Loaded<T>>;
  /** Starts reading a stage that isn't kept yet (a stage tab hovered or focused), so a click finds it there. */
  prefetch(id: string): void;
  subscribe(listener: () => void): () => void;
  version(): number;
}

// Opening a stage reads it again unless it was read this recently (just prefetched, say).
const RECENT_MS = 5_000;
// Loads quicker than this never dim the page.
const STALE_DELAY_MS = 150;

export function createStageCache<T>(fetcher: (id: string) => Promise<T>): StageCache<T> {
  const entries = new Map<string, Loaded<T>>();
  const inflight = new Map<string, Promise<Loaded<T>>>();
  // Each stage's newest stored request, so an older answer arriving late never replaces a newer one.
  const stored = new Map<string, number>();
  const listeners = new Set<() => void>();
  let requests = 0;
  let version = 0;

  function load(id: string, fresh = false): Promise<Loaded<T>> {
    if (!fresh) {
      const pending = inflight.get(id);
      if (pending) return pending;
      const entry = entries.get(id);
      if (entry && Date.now() - entry.loadedAt.getTime() < RECENT_MS) return Promise.resolve(entry);
    }
    const n = ++requests;
    const request = fetcher(id)
      .then((data) => {
        if ((stored.get(id) ?? 0) < n) {
          stored.set(id, n);
          entries.set(id, { id, data, loadedAt: new Date() });
          version++;
          listeners.forEach((l) => l());
        }
        return entries.get(id)!;
      })
      .finally(() => {
        if (inflight.get(id) === request) inflight.delete(id);
      });
    inflight.set(id, request);
    return request;
  }

  return {
    get: (id) => entries.get(id),
    load,
    prefetch: (id) => {
      if (!entries.has(id)) load(id).catch(() => {});
    },
    subscribe: (listener) => {
      listeners.add(listener);
      return () => listeners.delete(listener);
    },
    version: () => version,
  };
}

/**
 * The selected stage's data from `cache`, read when the stage is selected. Until it's there the
 * last stage shown stays on screen (`shownId` says which), and `stale` turns on if it takes longer
 * than a moment, so the page can dim it rather than blank it. Whatever describes the shown data
 * must come from `shownId`, not the selection.
 */
export function useStageCache<T>(cache: StageCache<T>, id: string | null) {
  useSyncExternalStore(cache.subscribe, cache.version);
  const entry = id ? cache.get(id) : undefined;
  const [error, setError] = useState<{ id: string; message: string } | null>(null);
  const [loading, setLoading] = useState(false);
  const failed = error !== null && error.id === id;

  // Derived during render (React's pattern for state that follows props): the last stage that was in.
  const [last, setLast] = useState<Loaded<T> | null>(null);
  if (entry && entry !== last) setLast(entry);
  // A stage that failed to load doesn't keep another's data under its tab.
  const shown = entry ?? (failed ? null : last);

  const waiting = id !== null && !entry && !failed;
  const [slow, setSlow] = useState(false);
  useEffect(() => {
    if (!waiting) {
      setSlow(false);
      return;
    }
    const timer = window.setTimeout(() => setSlow(true), STALE_DELAY_MS);
    return () => window.clearTimeout(timer);
  }, [waiting]);

  const current = useRef(id);
  current.current = id;

  const load = useCallback(
    async (fresh: boolean) => {
      const target = current.current;
      if (!target) {
        setLoading(false);
        return;
      }
      setLoading(true);
      try {
        await cache.load(target, fresh);
        setError((e) => (e?.id === target ? null : e));
      } catch (e) {
        if (current.current === target) setError({ id: target, message: errorMessage(e) });
      } finally {
        // A stage picked while this was on its way has its own load.
        if (current.current === target) setLoading(false);
      }
    },
    [cache],
  );

  useEffect(() => {
    void load(false);
  }, [id, load]);

  const reload = useCallback(() => load(true), [load]);
  const clearError = useCallback(() => setError(null), []);

  return {
    data: shown?.data ?? null,
    shownId: shown?.id ?? null,
    loadedAt: shown?.loadedAt ?? null,
    stale: waiting && slow && shown !== null,
    loading,
    error: failed ? error.message : null,
    clearError,
    reload,
  };
}
