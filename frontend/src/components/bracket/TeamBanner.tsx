import { API_URL } from "../../api/client";
import type { Team } from "../../api/types";
import "./TeamBanner.css";

/** A small crop of a team's banner; an empty box for a team without one or an empty slot, so rows line up. */
export function TeamBanner({ team }: { team?: Team }) {
  return (
    <span className="team-banner">
      {team?.banner_url && <img src={`${API_URL}${team.banner_url}`} alt="" loading="lazy" />}
    </span>
  );
}
