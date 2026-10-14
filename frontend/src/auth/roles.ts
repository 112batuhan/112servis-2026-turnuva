import type { Role } from "../api/types";

// Mirrors the backend's Role::has_at_least.
const RANK: Record<Role, number> = { basic: 0, streamer: 1, referee: 2, map_pooler: 3, host: 4 };

export const ROLES: Role[] = ["host", "map_pooler", "referee", "streamer", "basic"];

export const ROLE_LABELS: Record<Role, string> = {
  host: "Host",
  map_pooler: "Map Pooler",
  referee: "Hakem",
  streamer: "Yayıncı",
  basic: "Temel",
};

export function hasRole(user: { role: Role } | null | undefined, min: Role): boolean {
  return user ? RANK[user.role] >= RANK[min] : false;
}

/** Mirrors the backend's Role::can_play: streamers are the only staff who also play. */
export function canPlay(user: { role: Role } | null | undefined): boolean {
  return user?.role === "basic" || user?.role === "streamer";
}
