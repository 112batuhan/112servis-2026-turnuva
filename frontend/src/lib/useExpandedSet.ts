import { useCallback, useEffect, useState } from "react";
import { readJson, writeJson } from "./storage";

/** A set of ids, remembered in localStorage. Starts empty. */
export function useStoredSet(storageKey: string) {
  const [ids, setIds] = useState(() => new Set(readJson<string[]>(storageKey, [])));

  useEffect(() => {
    writeJson(storageKey, [...ids]);
  }, [storageKey, ids]);

  const toggle = useCallback((id: string) => {
    setIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const has = useCallback((id: string) => ids.has(id), [ids]);

  return { has, toggle };
}

/** A set of expanded section ids, remembered in localStorage. Everything starts collapsed. */
export function useExpandedSet(storageKey: string) {
  const { has, toggle } = useStoredSet(storageKey);
  return { isExpanded: has, toggle };
}
