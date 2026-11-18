import { useCallback, useEffect, useState } from "react";
import { fetchAutorefLogs } from "../../../api/referee";
import type { AutorefLogEntry } from "../../../api/types";
import { useLive } from "../../../live/LiveContext";

/**
 * The match's auto referee log for the referee page's log tab: what's saved, then every line the
 * server sends as the bot writes it (`autoref_log`, to staff watching the match). Loaded again when
 * the match's rooms change and after a reconnect (`bracket`), so nothing written meanwhile is missed.
 */
export function useAutorefLog(matchId: string, roomsKey: string): AutorefLogEntry[] {
  const { subscribe } = useLive();
  const [lines, setLines] = useState<AutorefLogEntry[]>([]);

  const load = useCallback(() => {
    fetchAutorefLogs(matchId)
      .then(setLines)
      .catch(() => {
        // The log is for debugging: a failed load leaves what's shown, and the next reload retries.
      });
  }, [matchId]);

  useEffect(() => load(), [load, roomsKey]);

  useEffect(
    () =>
      subscribe((event) => {
        if (event.type === "autoref_log" && event.match_id === matchId) {
          setLines((prev) =>
            prev.some((l) => l.id === event.entry.id) ? prev : [...prev, event.entry],
          );
        } else if (event.type === "bracket") {
          load();
        }
      }),
    [subscribe, matchId, load],
  );

  return lines;
}
