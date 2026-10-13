import type { Role } from "../api/types";

// Mirrors the backend's Role::has_at_least.
const RANK: Record<Role, number> = { basic: 0, referee: 1, map_pooler: 2, host: 3 };

export const ROLES: Role[] = ["host", "map_pooler", "referee", "basic"];

export const ROLE_LABELS: Record<Role, string> = {
  host: "Host",
  map_pooler: "Map Pooler",
  referee: "Hakem",
  basic: "Temel",
};

export function hasRole(user: { role: Role } | null | undefined, min: Role): boolean {
  return user ? RANK[user.role] >= RANK[min] : false;
}
