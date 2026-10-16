import { del, get, post, put } from "./client";
import type { BracketSide, MapResultRow, NextMatch, Player, PublicBracket, StaffBracket } from "./types";

export const fetchPublicBracket = () => get<PublicBracket>("/api/public/bracket");

// Referees and up.
export const fetchBracket = () => get<StaffBracket>("/api/bracket");

// Host-only.
export const createBracketMatch = (side: BracketSide, x: number, y: number) =>
  post<{ id: string }>("/api/bracket", { side, pos_x: x, pos_y: y });

export interface BracketMatchUpdate {
  side: BracketSide;
  number: number;
  starts_at: string | null;
  team1_id: string | null;
  team2_id: string | null;
  winner_to: NextMatch | null;
  loser_to: NextMatch | null;
  stage_id: string | null;
}

export const updateBracketMatch = (id: string, update: BracketMatchUpdate) =>
  put<void>(`/api/bracket/${id}`, update);
export const moveBracketMatch = (id: string, x: number, y: number) =>
  put<void>(`/api/bracket/${id}/position`, { pos_x: x, pos_y: y });
export const deleteBracketMatch = (id: string) => del(`/api/bracket/${id}`);
/** Everyone a match's stream can be given to. */
export const fetchStreamers = () => get<Player[]>("/api/streamers");

// The match's referee, or a host. Linking, notes, concluding and the referee itself go
// through the shared match routes in ./matches.
/** `maps`, from the referee page, replaces the map results the score was counted from. */
export const setBracketScore = (
  id: string,
  team1Score: number | null,
  team2Score: number | null,
  maps?: MapResultRow[],
) => put<void>(`/api/bracket/${id}/score`, { team1_score: team1Score, team2_score: team2Score, maps });

/** Concludes a match by forfeit: `slot` (1 or 2) is the team that forfeited, the other one moves on. */
export const forfeitMatch = (id: string, slot: 1 | 2) => post<void>(`/api/bracket/${id}/forfeit`, { slot });

/** Awards a map (or, with no map, a point) to one of the match's teams by hand. */
export const addManualResult = (id: string, teamId: string, mapId: string | null, note: string) =>
  post<{ id: string }>(`/api/bracket/${id}/results`, { team_id: teamId, map_id: mapId, note });
export const removeManualResult = (id: string, resultId: string) => del(`/api/bracket/${id}/results/${resultId}`);

/** Marks a pool map as banned by one of the match's two teams, or moves the ban to the other one. */
export const setMapBan = (id: string, mapId: string, teamId: string) =>
  put<void>(`/api/bracket/${id}/bans/${mapId}`, { team_id: teamId });
export const removeMapBan = (id: string, mapId: string) => del(`/api/bracket/${id}/bans/${mapId}`);
/** Marks a pool map as picked by one of the match's two teams, or moves the pick to the other one. */
export const setMapPick = (id: string, mapId: string, teamId: string) =>
  put<void>(`/api/bracket/${id}/picks/${mapId}`, { team_id: teamId });
export const removeMapPick = (id: string, mapId: string) => del(`/api/bracket/${id}/picks/${mapId}`);

/** A streamer passes their own id to take a match or null to give it up; hosts can pick anyone. */
export const setStreamer = (id: string, streamerId: string | null) =>
  put<void>(`/api/bracket/${id}/streamer`, { streamer_id: streamerId });
