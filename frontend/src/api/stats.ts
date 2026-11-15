import { get } from "./client";
import type { QualifierPlace, StageActivity, StageStats } from "./types";

export const fetchStageActivity = () => get<StageActivity[]>("/api/public/stats");
export const fetchStageStats = (stageId: string) => get<StageStats>(`/api/public/stats/${stageId}`);
/** Each team's place in the qualifier by team id, once the qualifier's pool is published. */
export const fetchQualifierPlaces = () =>
  get<Record<string, QualifierPlace>>("/api/public/qualifier-places");
