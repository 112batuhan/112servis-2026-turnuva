import { API_URL, get, post, put } from "./client";
import type { RefereeRoom, RoomChat, RoomCommand } from "./types";

// Referees and up.
export const fetchRefereeRoom = (matchId: string) => get<RefereeRoom>(`/api/matches/${matchId}/room`);
/** The saved chat of the match's room, for moderation. */
export const fetchRoomChat = (matchId: string) => get<RoomChat[]>(`/api/matches/${matchId}/chat`);

// Any referee, or a host: what the room keeps, shared by everyone on it.
export const setRoomOptions = (
  matchId: string,
  options: { score_mode: number; no_fail: boolean; auto_place: boolean; auto_kick: boolean },
) => put<void>(`/api/matches/${matchId}/room-options`, options);
/** Reads osu!'s record of the rooms in place of the chat's maps; returns when. */
export const syncFromOsu = (matchId: string) => post<{ osu_synced_at: string }>(`/api/matches/${matchId}/osu-sync`);
/** Whose pick is next, until the next ban or pick; null goes back to the rules. */
export const setNextPick = (matchId: string, teamId: string | null) =>
  put<void>(`/api/matches/${matchId}/next-pick`, { team_id: teamId });
/** Leaves a finished map out of the score ("Sayma"), or counts it again. */
export const setIgnoredMap = (matchId: string, key: string, ignored: boolean) =>
  put<void>(`/api/matches/${matchId}/ignored-maps`, { key, ignored });

// The room's bot (backend/src/bancho).
/** The least busy bot with room for one makes the match's room (or a new one) and sets it up. */
export const createRoom = (matchId: string) => post<{ mp_id: number }>(`/api/matches/${matchId}/room/create`);
export const roomCommand = (matchId: string, command: RoomCommand) =>
  post<void>(`/api/matches/${matchId}/room/command`, command);
/** Saves the score as the bots count it, now. */
export const saveRoomScore = (matchId: string) => post<void>(`/api/matches/${matchId}/room/save-score`);

/** The referee's own Bancho IRC relay's WebSocket (backend/src/handlers/irc.rs). */
export function ircSocketUrl(): string {
  const base = API_URL || window.location.origin;
  return `${base.replace(/^http/, "ws")}/api/irc`;
}
