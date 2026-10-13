/** Moves `id` to where `targetId` currently is. Returns null when nothing changes. */
export function moveToTarget(ids: string[], id: string, targetId: string): string[] | null {
  const to = ids.indexOf(targetId);
  return to < 0 ? null : moveToIndex(ids, id, to);
}

/** Moves `id` by `delta` places (-1 = earlier, 1 = later). Returns null when nothing changes. */
export function moveBy(ids: string[], id: string, delta: number): string[] | null {
  return moveToIndex(ids, id, ids.indexOf(id) + delta);
}

function moveToIndex(ids: string[], id: string, to: number): string[] | null {
  const from = ids.indexOf(id);
  if (from < 0 || to < 0 || to >= ids.length || from === to) return null;
  const next = [...ids];
  next.splice(from, 1);
  next.splice(to, 0, id);
  return next;
}

/** Re-sorts `items` to follow `ids`. */
export function sortByIds<T extends { id: string }>(items: T[], ids: string[]): T[] {
  const byId = new Map(items.map((item) => [item.id, item]));
  return ids.map((id) => byId.get(id)).filter((item): item is T => item !== undefined);
}
