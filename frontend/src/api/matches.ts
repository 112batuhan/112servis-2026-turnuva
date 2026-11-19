import { del, get, patch, post, put } from "./client";
import type {
  Player,
  PlayerAction,
  PublicMatch,
  PublicQualifiers,
  RoomChat,
  StaffQualifiers,
} from "./types";

export const fetchPublicQualifiers = () => get<PublicQualifiers>("/api/public/qualifiers");
/** Signs the caller's team up for a lobby, or moves it there. */
export const joinLobby = (matchId: string) => put<void>("/api/me/qualifier", { match_id: matchId });
export const leaveLobby = () => del("/api/me/qualifier");

// Staff.
export const fetchQualifiers = () => get<StaffQualifiers>("/api/qualifiers");

// Host-only.
export const createLobby = (startsAt: string) =>
  post<void>("/api/qualifiers", { starts_at: startsAt });
export const rescheduleLobby = (id: string, startsAt: string) =>
  patch<void>(`/api/qualifiers/${id}`, { starts_at: startsAt });
export const deleteLobby = (id: string) => del(`/api/qualifiers/${id}`);
export const assignTeam = (teamId: string, matchId: string) =>
  put<void>(`/api/teams/${teamId}/qualifier`, { match_id: matchId });
export const unassignTeam = (teamId: string) => del(`/api/teams/${teamId}/qualifier`);
/** Everyone a match can be given to. */
export const fetchReferees = () => get<Player[]>("/api/referees");

// Referees and hosts.
/** A referee passes their own id to take a match or null to give it up; hosts can pick anyone. */
export const setReferee = (matchId: string, refereeId: string | null) =>
  put<void>(`/api/matches/${matchId}/referee`, { referee_id: refereeId });
/** Adds an osu! multiplayer room, by link or room id. */
export const addMatchRoom = (matchId: string, link: string) =>
  post<void>(`/api/matches/${matchId}/rooms`, { link });
export const removeMatchRoom = (matchId: string, mpId: number) =>
  del(`/api/matches/${matchId}/rooms/${mpId}`);
export const setMatchNotes = (matchId: string, notes: string) =>
  put<void>(`/api/matches/${matchId}/notes`, { notes });
export const concludeMatch = (matchId: string) => post<void>(`/api/matches/${matchId}/conclude`);
/** Host-only. */
export const reopenMatch = (matchId: string) => post<void>(`/api/matches/${matchId}/reopen`);

// Anyone: the public match page.
export const fetchPublicMatch = (matchId: string) =>
  get<PublicMatch>(`/api/public/matches/${matchId}`);
/** The saved chat of each of the match's rooms; new lines arrive over the live socket. */
export const fetchPublicChat = (matchId: string) =>
  get<RoomChat[]>(`/api/public/matches/${matchId}/chat`);

// The match's players: their room page (pages/public/match/room).
/** A ban or pick, a warmup, an abort, a report or the emergency stop; the room's bot says no if it isn't theirs to do now. */
export const playerAction = (matchId: string, action: PlayerAction) =>
  post<void>(`/api/matches/${matchId}/player`, action);
