// Role privilege ladder, mirroring the backend's Role::has_at_least.
const RANK = { basic: 0, map_pooler: 1, host: 2 };

// True if the user's role is at least `min`.
export function hasRole(user, min) {
  return (RANK[user?.role] ?? -1) >= (RANK[min] ?? 99);
}
