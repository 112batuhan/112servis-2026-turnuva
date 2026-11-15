import { del, get, post } from "./client";
import type { DevAccount } from "./types";

// Only mounted when the backend runs with DEV_LOGIN=true; otherwise these 404.
export const fetchDevAccounts = () => get<DevAccount[]>("/api/dev/accounts");
export const devLoginAs = (userId: string) => post<void>(`/api/dev/login/${userId}`);
export const devSeed = () => post<void>("/api/dev/seed");
/** Reseeds as the tournament just after its qualifier: every lobby played, no bracket yet. */
export const devSeedQualifiers = () => post<void>("/api/dev/seed/qualifiers");
/** Reseeds with only a few teams and the dev staff: no lobby, score or bracket. */
export const devSeedTeams = () => post<void>("/api/dev/seed/teams");
export const devSeedLive = () => post<void>("/api/dev/seed/live");
export const devClear = () => del("/api/dev/seed");
