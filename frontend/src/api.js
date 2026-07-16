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

// Host-only: the full list of registered users for the admin panel.
export async function fetchUsers() {
  const res = await fetch(`${API_URL}/api/users`, { credentials: "include" });
  if (!res.ok) throw new Error(`Failed to load users (${res.status})`);
  return res.json();
}

// Host-only: change a user's role. Bumps their token_version server-side.
export async function updateUserRole(userId, role) {
  const res = await fetch(`${API_URL}/api/users/${userId}/role`, {
    method: "POST",
    credentials: "include",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ role }),
  });
  if (!res.ok) throw new Error(`Failed to update role (${res.status})`);
  return res.json();
}

// ---- map pools (map_pooler+) ----

// Shared request helper: sends cookies, throws the server's error text on failure,
// and returns null for 204 responses.
async function request(path, options = {}) {
  const res = await fetch(`${API_URL}${path}`, { credentials: "include", ...options });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || `Request failed (${res.status})`);
  }
  return res.status === 204 ? null : res.json();
}

const jsonBody = (method, data) => ({
  method,
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify(data),
});

export const fetchStages = () => request("/api/stages");
export const createStage = (name) => request("/api/stages", jsonBody("POST", { name }));
export const deleteStage = (id) => request(`/api/stages/${id}`, { method: "DELETE" });
export const fetchStage = (id) => request(`/api/stages/${id}`);
export const setStagePublished = (id, published) =>
  request(`/api/stages/${id}`, jsonBody("PATCH", { published }));

export const createCategory = (stageId, name, modifier) =>
  request(`/api/stages/${stageId}/categories`, jsonBody("POST", { name, modifier: modifier || null }));
export const deleteCategory = (id) => request(`/api/categories/${id}`, { method: "DELETE" });

// Global generic pool (shared across all stages).
export const addToGenericPool = (beatmapId) => request("/api/pool", jsonBody("POST", { beatmap_id: beatmapId }));
export const removeFromGenericPool = (beatmapId) => request(`/api/pool/${beatmapId}`, { method: "DELETE" });

// Per-stage categorised placements.
export const categorize = (stageId, beatmapId, categoryId) =>
  request(`/api/stages/${stageId}/entries`, jsonBody("POST", { beatmap_id: beatmapId, category_id: categoryId }));
export const moveEntry = (entryId, categoryId) =>
  request(`/api/entries/${entryId}`, jsonBody("PATCH", { category_id: categoryId }));
export const deleteEntry = (entryId) => request(`/api/entries/${entryId}`, { method: "DELETE" });

// Public (unauthenticated) — published stages only.
export const fetchPublicStages = () => request("/api/public/stages");
export const fetchPublicStage = (id) => request(`/api/public/stages/${id}`);
