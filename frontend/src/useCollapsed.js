import { useEffect, useState } from "react";

// Tracks a set of collapsed section ids, persisted to localStorage under `storageKey`
// so a user's expand/collapse choices survive reloads. Returns [collapsedSet, toggle].
export function useCollapsed(storageKey) {
  const [collapsed, setCollapsed] = useState(() => {
    try {
      const raw = localStorage.getItem(storageKey);
      return new Set(raw ? JSON.parse(raw) : []);
    } catch {
      return new Set();
    }
  });

  useEffect(() => {
    try {
      localStorage.setItem(storageKey, JSON.stringify([...collapsed]));
    } catch {
      // Ignore write failures (private mode / quota) — collapsing still works in-memory.
    }
  }, [storageKey, collapsed]);

  const toggle = (id) =>
    setCollapsed((cur) => {
      const next = new Set(cur);
      next.has(id) ? next.delete(id) : next.add(id);
      return next;
    });

  return [collapsed, toggle];
}
