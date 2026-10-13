import type { Eligibility } from "../api/types";

export const ELIGIBILITY_LABELS: Record<Eligibility, string> = {
  eligible: "Eligible",
  in_team: "Already in a team",
  unranked: "Unranked",
  over_limit: "Over the rank limit",
};

export const eligibilityBadgeClass = (e: Eligibility) => (e === "eligible" ? "badge badge-success" : "badge badge-warning");
