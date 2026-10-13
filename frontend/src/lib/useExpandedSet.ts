import { useCallback, useEffect, useState } from "react";
import { readJson, writeJson } from "./storage";

/** A set of expanded section ids, remembered in localStorage. Everything starts collapsed. */
export function useExpandedSet(storageKey: string) {
  const [expanded, setExpanded] = useState(() => new Set(readJson<string[]>(storageKey, [])));

  useEffect(() => {
    writeJson(storageKey, [...expanded]);
  }, [storageKey, expanded]);

  const toggle = useCallback((id: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const isExpanded = useCallback((id: string) => expanded.has(id), [expanded]);

  return { isExpanded, toggle };
}
