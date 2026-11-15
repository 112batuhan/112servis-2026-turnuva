import { del, get, put } from "./client";
import type { MatchPickem, PickemMode, PickemOverview } from "./types";

export const fetchPickem = () => get<PickemOverview>("/api/public/pickem");
export const fetchMatchPickem = (matchId: string) =>
  get<MatchPickem | null>(`/api/public/pickem/matches/${matchId}`);
/** The winner's maps are what the stage's best of needs; only the loser's are sent. */
export const predictMatch = (matchId: string, teamId: string, loserScore: number) =>
  put<void>(`/api/pickem/${matchId}`, { team_id: teamId, loser_score: loserScore });
export const withdrawPrediction = (matchId: string) => del(`/api/pickem/${matchId}`);
export const setStagePickemMode = (stageId: string, mode: PickemMode) =>
  put<void>(`/api/stages/${stageId}/pickem`, { mode });
