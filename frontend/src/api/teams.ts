import { del, get, patch, post, put, putFile } from "./client";
import type { AdminTeamPlayer, PlayerResult, RankRefresh, TeamList, TeamStatus } from "./types";

export const searchPlayers = (q: string) =>
  get<PlayerResult[]>(`/api/players/search?q=${encodeURIComponent(q)}`);

export const fetchTeamStatus = () => get<TeamStatus>("/api/me/team");
export const sendInvite = (userId: string, teamName: string) =>
  post<void>("/api/invites", { user_id: userId, team_name: teamName });
export const acceptInvite = (inviteId: string) => post<void>(`/api/invites/${inviteId}/accept`);
/** Cancels a sent invite or declines a received one. */
export const deleteInvite = (inviteId: string) => del(`/api/invites/${inviteId}`);

export const renameMyTeam = (name: string) => patch<void>("/api/me/team", { name });
export const disbandMyTeam = () => del("/api/me/team");
export const uploadMyBanner = (file: File) => putFile("/api/me/team/banner", file);
export const removeMyBanner = () => del("/api/me/team/banner");

export const fetchPublicTeams = () => get<TeamList>("/api/public/teams");

// Host-only.
export const fetchTeams = () => get<TeamList<AdminTeamPlayer>>("/api/teams");
export const renameTeam = (teamId: string, name: string) => patch<void>(`/api/teams/${teamId}`, { name });
export const disbandTeam = (teamId: string) => del(`/api/teams/${teamId}`);
export const removeTeamBanner = (teamId: string) => del(`/api/teams/${teamId}/banner`);
/** Refreshes every team member's rank from osu! now instead of waiting for the daily run. */
export const refreshRanks = () => post<RankRefresh>("/api/ranks/refresh");
export const setTeamsFrozen = (frozen: boolean) => put<void>("/api/settings/teams-frozen", { frozen });
