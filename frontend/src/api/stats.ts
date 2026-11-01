import { get } from "./client";
import type { StageActivity, StageStats } from "./types";

export const fetchStageActivity = () => get<StageActivity[]>("/api/public/stats");
export const fetchStageStats = (stageId: string) => get<StageStats>(`/api/public/stats/${stageId}`);
