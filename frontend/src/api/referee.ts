import { API_URL, get, post, put } from "./client";
import type { OsuRoomGames, RefereeRoom, RoomChat } from "./types";

// Referees and up.
export const fetchRefereeRoom = (matchId: string) => get<RefereeRoom>(`/api/matches/${matchId}/room`);
/** The saved chat of the match's room, for moderation. */
export const fetchRoomChat = (matchId: string) => get<RoomChat[]>(`/api/matches/${matchId}/chat`);
/** Each of the match's rooms as osu! recorded it, for when the chat missed something. */
export const fetchOsuGames = (matchId: string) => get<OsuRoomGames[]>(`/api/matches/${matchId}/osu-games`);

// Any referee, or a host: what the referee page keeps, shared by everyone on it.
export const setRoomOptions = (
  matchId: string,
  options: { score_mode: number; no_fail: boolean; auto_place: boolean; auto_kick: boolean },
) => put<void>(`/api/matches/${matchId}/room-options`, options);
/** The results were synced from osu!'s record of the rooms just now; returns when. */
export const markOsuSynced = (matchId: string) => post<{ osu_synced_at: string }>(`/api/matches/${matchId}/osu-sync`);
/** Whose pick is next, until the next ban or pick; null goes back to the rules. */
export const setNextPick = (matchId: string, teamId: string | null) =>
  put<void>(`/api/matches/${matchId}/next-pick`, { team_id: teamId });
/** Leaves a finished map out of the score ("Sayma"), or counts it again. */
export const setIgnoredMap = (matchId: string, key: string, ignored: boolean) =>
  put<void>(`/api/matches/${matchId}/ignored-maps`, { key, ignored });

/** The Bancho IRC relay's WebSocket (backend/src/handlers/irc.rs). */
export function ircSocketUrl(): string {
  const base = API_URL || window.location.origin;
  return `${base.replace(/^http/, "ws")}/api/irc`;
}
