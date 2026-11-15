import { API_URL, post } from "./client";
import type { User } from "./types";

// OAuth needs a full browser navigation, not a fetch.
export function loginWithOsu() {
  window.location.href = `${API_URL}/auth/osu`;
}

export function linkDiscord() {
  window.location.href = `${API_URL}/auth/discord/link`;
}

/** The signed-in user, or null when signed out. */
export async function fetchCurrentUser(): Promise<User | null> {
  const res = await fetch(`${API_URL}/api/me`, { credentials: "include" });
  return res.ok ? res.json() : null;
}

export const logout = () => post<void>("/auth/logout");
