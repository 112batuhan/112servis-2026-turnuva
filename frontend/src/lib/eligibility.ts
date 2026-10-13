import type { Eligibility, RankWarning } from "../api/types";

export const ELIGIBILITY_LABELS: Record<Eligibility, string> = {
  eligible: "Uygun",
  not_player: "Yetkili",
  in_team: "Zaten bir takımda",
  unranked: "Unranked",
  over_limit: "Rank limitini aşıyor",
};

export const eligibilityBadgeClass = (e: Eligibility) =>
  e === "eligible" ? "badge badge-success" : "badge badge-warning";

export const RANK_WARNING_LABELS: Record<RankWarning, string> = {
  over_limit: "Rank limitini aşıyor",
  unranked: "Bir oyuncunun rank'ı yok",
};
