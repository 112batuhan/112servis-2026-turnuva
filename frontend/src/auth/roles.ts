import type { Role } from "../api/types";

// Mirrors the backend's role.rs. A user holds any set of roles; none means a regular player.
type WithRoles = { roles: Role[] } | null | undefined;

export const ROLES: Role[] = ["host", "map_pooler", "referee", "streamer"];

export const ROLE_LABELS: Record<Role, string> = {
  host: "Host",
  map_pooler: "Map Pooler",
  referee: "Hakem",
  streamer: "Yayıncı",
};

/** What a user with no roles is shown as. */
export const PLAYER_LABEL = "Oyuncu";

/** Mirrors Roles::allows: may do what the role may, by holding it or being a host. */
export function hasRole(user: WithRoles, role: Role): boolean {
  return !!user && (user.roles.includes(role) || user.roles.includes("host"));
}

/**
 * Mirrors Roles::has: holds the role itself. Taking a referee or streamer slot (and being offered
 * one) needs this; a host who referees holds Referee too.
 */
export function holdsRole(user: WithRoles, role: Role): boolean {
  return !!user && user.roles.includes(role);
}

/** Any staff role: the admin pages open for them. */
export function isStaff(user: WithRoles): boolean {
  return !!user && user.roles.length > 0;
}

/** Mirrors Roles::can_play: streamers are the only staff who also play. */
export function canPlay(user: WithRoles): boolean {
  return !!user && user.roles.every((r) => r === "streamer");
}

/** Who can open an admin page: holders of a role (hosts always), or any staff. */
export type Access = Role | "staff";

export function canAccess(user: WithRoles, access: Access): boolean {
  return access === "staff" ? isStaff(user) : hasRole(user, access);
}
