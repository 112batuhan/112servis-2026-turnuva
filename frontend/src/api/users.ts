import { get, put } from "./client";
import type { AdminUser, Role } from "./types";

// Host-only endpoints.
export const fetchUsers = () => get<AdminUser[]>("/api/users");

/** Replaces a user's roles; it applies on their next request. Hosts can't drop their own Host. */
export const updateUserRoles = (userId: string, roles: Role[]) =>
  put<AdminUser>(`/api/users/${userId}/roles`, { roles });
