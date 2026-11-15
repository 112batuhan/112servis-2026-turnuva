import type { Eligibility, RankWarning } from "../api/types";

export const ELIGIBILITY_LABELS: Record<Eligibility, string> = {
  eligible: "Uygun",
  not_player: "Yetkili",
  banned: "Banlı",
  in_team: "Zaten bir takımda",
  unranked: "Unranked",
  over_limit: "Limiti aşıyor",
};

export const eligibilityBadgeClass = (e: Eligibility) =>
  e === "eligible" ? "badge badge-success" : "badge badge-warning";

/** On `/admin/teams`, for a team one of whose players the hosts banned (staff-only: the public list doesn't say). */
export const BANNED_PLAYER_WARNING = "Banlanmış oyunculu takım";

export const RANK_WARNING_LABELS: Record<RankWarning, string> = {
  over_limit: "Takım değeri limiti aşıyor",
  unranked: "Bir oyuncunun rank'ı yok",
};
