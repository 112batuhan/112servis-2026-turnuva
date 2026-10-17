import { del, get, post, put } from "./client";
import type { DiscordSettings, DiscordView, Subscriptions } from "./types";

// Host-only: the bot's settings on /admin/discord.
export const fetchDiscord = () => get<DiscordView>("/api/discord");
export const saveDiscordSettings = (settings: DiscordSettings) => put<void>("/api/discord/settings", settings);
/** Rechecks who's in the server, everyone's roles and every match channel. */
export const syncDiscord = () => post<void>("/api/discord/sync");
/** Switches the bot off (disconnects, drops its waiting jobs) or back on (reconnects and syncs everything). */
export const setDiscordActive = (active: boolean) => put<void>("/api/discord/active", { active });

export const fetchSubscriptions = () => get<Subscriptions>("/api/me/subscriptions");
/** Pings the signed-in user on Discord when the bracket match is about to start. */
export const subscribe = (matchId: string) => put<void>(`/api/bracket/${matchId}/subscription`, {});
export const unsubscribe = (matchId: string) => del(`/api/bracket/${matchId}/subscription`);

/** The match's referee pings one of its players in the match's Discord channel. */
export const pingPlayer = (matchId: string, userId: string) =>
  post<void>(`/api/matches/${matchId}/ping`, { user_id: userId });
