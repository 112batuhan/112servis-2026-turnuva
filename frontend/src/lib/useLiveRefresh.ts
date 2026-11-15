import { useEffect, useRef } from "react";
import type { BracketMatch } from "../api/types";
import { useLive } from "../live/LiveContext";

/** How often a page with a match on refetches the bracket. */
export const LIVE_REFRESH_MS = 30_000;
/** A match this close to its start already counts, so its room and first maps show up without a reload. */
const LEAD_MS = 10 * 60_000;

/** Whether any match is being played or about to be: unconcluded, and due within LEAD_MS. */
export function hasMatchOn(matches: BracketMatch[], now = Date.now()): boolean {
  return matches.some(
    (m) =>
      !m.concluded_at && m.starts_at !== null && new Date(m.starts_at).getTime() <= now + LEAD_MS,
  );
}

/**
 * The fallback for when the live socket is down (see live/LiveContext.tsx): calls `refresh` every
 * LIVE_REFRESH_MS while a match is on, so running scores, new rooms and results still reach the
 * page. Paused while the tab is hidden, and run once when it's shown again.
 */
export function useLiveRefresh(matches: BracketMatch[], refresh: () => unknown) {
  const refreshRef = useRef(refresh);
  refreshRef.current = refresh;
  const { connected } = useLive();
  const on = !connected && hasMatchOn(matches);

  useEffect(() => {
    if (!on) return;
    let timer: number | undefined;
    const start = () => {
      window.clearInterval(timer);
      timer = window.setInterval(() => refreshRef.current(), LIVE_REFRESH_MS);
    };
    const onVisibility = () => {
      if (document.visibilityState === "visible") {
        refreshRef.current();
        start();
      } else {
        window.clearInterval(timer);
      }
    };
    if (document.visibilityState === "visible") start();
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      window.clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisibility);
    };
  }, [on]);
}
