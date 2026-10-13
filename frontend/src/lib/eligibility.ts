import type { Eligibility, RankWarning } from "../api/types";

export const ELIGIBILITY_LABELS: Record<Eligibility, string> = {
  eligible: "Eligible",
  not_player: "Staff",
  in_team: "Already in a team",
  unranked: "Unranked",
  over_limit: "Over the rank limit",
};

export const eligibilityBadgeClass = (e: Eligibility) =>
  e === "eligible" ? "badge badge-success" : "badge badge-warning";

export const RANK_WARNING_LABELS: Record<RankWarning, string> = {
  over_limit: "Over the rank limit",
  unranked: "A player has no rank",
};
