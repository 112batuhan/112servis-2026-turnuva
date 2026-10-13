import { del, get, post } from "./client";
import type { DevAccount } from "./types";

// Only mounted when the backend runs with DEV_LOGIN=true; otherwise these 404.
export const fetchDevAccounts = () => get<DevAccount[]>("/api/dev/accounts");
export const devLoginAs = (userId: string) => post<void>(`/api/dev/login/${userId}`);
export const devSeed = () => post<void>("/api/dev/seed");
export const devClear = () => del("/api/dev/seed");
