import { API_URL, get } from "./client";
import type { OsuRoomGames, RefereeRoom, RoomChat } from "./types";

// Referees and up.
export const fetchRefereeRoom = (matchId: string) => get<RefereeRoom>(`/api/matches/${matchId}/room`);
/** The saved chat of the match's room, for moderation. */
export const fetchRoomChat = (matchId: string) => get<RoomChat[]>(`/api/matches/${matchId}/chat`);
/** Each of the match's rooms as osu! recorded it, for when the chat missed something. */
export const fetchOsuGames = (matchId: string) => get<OsuRoomGames[]>(`/api/matches/${matchId}/osu-games`);

/** The Bancho IRC relay's WebSocket (backend/src/handlers/irc.rs). */
export function ircSocketUrl(): string {
  const base = API_URL || window.location.origin;
  return `${base.replace(/^http/, "ws")}/api/irc`;
}
