// `??` (not `||`) so an explicitly empty VITE_API_URL (same-origin deployments,
// e.g. the Docker image which serves the frontend from the Rust backend) is preserved.
export const API_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8080";

// Full navigation, not a fetch, since OAuth redirects happen at the browser level.
export function loginWithOsu() {
  window.location.href = `${API_URL}/auth/osu`;
}

export function linkDiscord() {
  window.location.href = `${API_URL}/auth/discord/link`;
}

export async function fetchCurrentUser() {
  const res = await fetch(`${API_URL}/api/me`, { credentials: "include" });
  if (!res.ok) return null;
  return res.json();
}

export async function logout() {
  await fetch(`${API_URL}/auth/logout`, { method: "POST", credentials: "include" });
}
