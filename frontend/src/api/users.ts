import { get, post } from "./client";
import type { AdminUser, Role } from "./types";

// Host-only endpoints.
export const fetchUsers = () => get<AdminUser[]>("/api/users");

/** Changing a role also invalidates that user's existing sessions server-side. */
export const updateUserRole = (userId: string, role: Role) =>
  post<AdminUser>(`/api/users/${userId}/role`, { role });
