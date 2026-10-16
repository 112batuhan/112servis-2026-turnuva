import { API_URL } from "./client";

/** The live hub's WebSocket (handlers/live.rs): score pushes and referee-page presence. */
export function liveSocketUrl(): string {
  const base = API_URL || window.location.origin;
  return `${base.replace(/^http/, "ws")}/api/live`;
}
