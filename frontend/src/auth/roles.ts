import type { Role } from "../api/types";

// Mirrors the backend's Role::has_at_least.
const RANK: Record<Role, number> = { basic: 0, map_pooler: 1, host: 2 };

export const ROLES: Role[] = ["host", "map_pooler", "basic"];

export const ROLE_LABELS: Record<Role, string> = {
  host: "Host",
  map_pooler: "Map Pooler",
  basic: "Temel",
};

export function hasRole(user: { role: Role } | null | undefined, min: Role): boolean {
  return user ? RANK[user.role] >= RANK[min] : false;
}
