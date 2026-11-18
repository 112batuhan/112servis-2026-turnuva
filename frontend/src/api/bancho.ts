import { del, get, patch, post, put } from "./client";
import type { BotsView } from "./types";

// Host-only: the osu! accounts the site runs rooms with, on /admin/bancho.
export const fetchBots = () => get<BotsView>("/api/bancho/bots");
export const createBot = (username: string, ircPassword: string, active: boolean) =>
  post<{ id: string }>("/api/bancho/bots", { username, irc_password: ircPassword, active });
export const updateBot = (id: string, change: { irc_password?: string; active?: boolean }) =>
  patch<void>(`/api/bancho/bots/${id}`, change);
export const deleteBot = (id: string) => del(`/api/bancho/bots/${id}`);
/** Whether the auto referee opens match rooms by itself, 10 minutes before each match. */
export const setAutoRooms = (enabled: boolean) => put<void>("/api/bancho/auto-rooms", { enabled });
