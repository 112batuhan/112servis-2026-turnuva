import { del, get, put } from "./client";
import type { AdminUser, Role, UserBan } from "./types";

// Host-only endpoints.
export const fetchUsers = () => get<AdminUser[]>("/api/users");

/** Replaces a user's roles; it applies on their next request. Hosts can't drop their own Host. */
export const updateUserRoles = (userId: string, roles: Role[]) =>
  put<AdminUser>(`/api/users/${userId}/roles`, { roles });

/** Bans a user from taking part (teams, lobbies, reschedules, pick'em), or changes the reason. */
export const banUser = (userId: string, reason: string) => put<UserBan>(`/api/users/${userId}/ban`, { reason });

export const unbanUser = (userId: string) => del(`/api/users/${userId}/ban`);
