import { del, get, post, put } from "./client";
import type { BracketSide, NextMatch, Player, PublicBracket, StaffBracket } from "./types";

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
export const setBracketScore = (id: string, team1Score: number | null, team2Score: number | null) =>
  put<void>(`/api/bracket/${id}/score`, { team1_score: team1Score, team2_score: team2Score });

/** A streamer passes their own id to take a match or null to give it up; hosts can pick anyone. */
export const setStreamer = (id: string, streamerId: string | null) =>
  put<void>(`/api/bracket/${id}/streamer`, { streamer_id: streamerId });
