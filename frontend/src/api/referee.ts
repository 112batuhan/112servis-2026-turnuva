import { API_URL, get } from "./client";
import type { RefereeRoom, RoomChat } from "./types";

// Referees and up.
export const fetchRefereeRoom = (matchId: string) => get<RefereeRoom>(`/api/matches/${matchId}/room`);
/** The saved chat of the match's room, for moderation. */
export const fetchRoomChat = (matchId: string) => get<RoomChat[]>(`/api/matches/${matchId}/chat`);

/** The Bancho IRC relay's WebSocket (backend/src/handlers/irc.rs). */
export function ircSocketUrl(): string {
  const base = API_URL || window.location.origin;
  return `${base.replace(/^http/, "ws")}/api/irc`;
}
