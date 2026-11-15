import type { Role } from "../api/types";

// Mirrors the backend's role.rs. A user holds any set of roles; none means a regular player.
type WithRoles = { roles: Role[] } | null | undefined;

export const ROLES: Role[] = [
  "host",
  "map_pooler",
  "referee",
  "streamer",
  "graphics_designer",
  "pool_tester",
  "custom_mapper",
  "commentator",
];

export const ROLE_LABELS: Record<Role, string> = {
  host: "Host",
  map_pooler: "Map Pooler",
  referee: "Hakem",
  streamer: "Yayıncı",
  graphics_designer: "Grafik Tasarımcı",
  pool_tester: "Pool Tester",
  custom_mapper: "Custom Mapper",
  commentator: "Yorumcu",
};

/** Roles with no authority: they only list the user on the staff page, and their holders still play. */
const LISTED_ONLY: Role[] = ["graphics_designer", "commentator"];

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

/**
 * Any role with a part in the admin panel: everyone on the staff but graphics designers and
 * commentators, who are only listed on the staff page.
 */
export function isStaff(user: WithRoles): boolean {
  return !!user && user.roles.some((r) => !LISTED_ONLY.includes(r));
}

/** Mirrors Roles::can_play: streamers, graphics designers and commentators are the only staff who also play. */
export function canPlay(user: WithRoles): boolean {
  return !!user && user.roles.every((r) => r === "streamer" || LISTED_ONLY.includes(r));
}

/**
 * Mirrors Roles::can_view_pool: map poolers (and hosts) edit the map pool editor, pool testers and
 * custom mappers only look at it.
 */
export function canViewPool(user: WithRoles): boolean {
  return (
    hasRole(user, "map_pooler") ||
    holdsRole(user, "pool_tester") ||
    holdsRole(user, "custom_mapper")
  );
}

/** The roles that see the admin bracket tab; pool testers and custom mappers only get the map pool. */
const MATCH_STAFF: Role[] = ["host", "map_pooler", "referee", "streamer"];

/**
 * Who can open an admin page: holders of a role (hosts always), any staff, whoever can view the map
 * pool, or the staff who see the matches.
 */
export type Access = Role | "staff" | "pool" | "matches";

export function canAccess(user: WithRoles, access: Access): boolean {
  switch (access) {
    case "staff":
      return isStaff(user);
    case "pool":
      return canViewPool(user);
    case "matches":
      return !!user && user.roles.some((r) => MATCH_STAFF.includes(r));
    default:
      return hasRole(user, access);
  }
}
