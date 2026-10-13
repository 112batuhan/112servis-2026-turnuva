import { del, get, post } from "./client";
import type { AdminTeamPlayer, PlayerResult, Team, TeamStatus } from "./types";

export const searchPlayers = (q: string) =>
  get<PlayerResult[]>(`/api/players/search?q=${encodeURIComponent(q)}`);

export const fetchTeamStatus = () => get<TeamStatus>("/api/me/team");
export const sendInvite = (userId: string) => post<void>("/api/invites", { user_id: userId });
export const acceptInvite = (inviteId: string) => post<void>(`/api/invites/${inviteId}/accept`);
/** Cancels a sent invite or declines a received one. */
export const deleteInvite = (inviteId: string) => del(`/api/invites/${inviteId}`);

export const fetchPublicTeams = () => get<Team[]>("/api/public/teams");

// Host-only.
export const fetchTeams = () => get<Team<AdminTeamPlayer>[]>("/api/teams");
export const disbandTeam = (teamId: string) => del(`/api/teams/${teamId}`);
