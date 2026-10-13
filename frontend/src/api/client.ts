// `??` rather than `||`: an explicitly empty VITE_API_URL means "same origin"
// (the Docker image serves the frontend from the backend).
export const API_URL = import.meta.env.VITE_API_URL ?? "http://localhost:8080";

/** Sends cookies, throws the server's error text on failure, and returns undefined for 204s. */
export async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const res = await fetch(`${API_URL}${path}`, { credentials: "include", ...init });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(text || `Request failed (${res.status})`);
  }
  return (res.status === 204 ? undefined : await res.json()) as T;
}

function withJson(method: string, body: unknown): RequestInit {
  return {
    method,
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  };
}

export const get = <T>(path: string) => request<T>(path);
export const post = <T>(path: string, body?: unknown) =>
  request<T>(path, body === undefined ? { method: "POST" } : withJson("POST", body));
export const patch = <T>(path: string, body: unknown) => request<T>(path, withJson("PATCH", body));
export const del = (path: string) => request<void>(path, { method: "DELETE" });
/** Sends a file as the raw request body. */
export const putFile = (path: string, file: Blob) =>
  request<void>(path, { method: "PUT", headers: { "Content-Type": file.type }, body: file });

export function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}
