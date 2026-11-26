import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";
import { isOwnMatch, matchPagePath, roomPagePath } from "../lib/bracket";
import { isAdminPath } from "../lib/adminTabs";
import { Icon } from "./Icon";
import "./MatchPageLink.css";

interface Props {
  matchId: string;
  /** Being played right now: the link says so and is lit. */
  live?: boolean;
  /**
   * "card" is a bare icon for the grid's cards and the calendar's chips, "row" an icon button for
   * the match panel, "text" a labelled small button for list rows, lobby cards and headers.
   */
  variant: "card" | "row" | "text";
  /**
   * The match's teams (a lobby's every team) and whether it's over: for one of its own players it
   * opens the room page instead, "Maç odası".
   */
  teamIds?: readonly (string | null | undefined)[];
  concluded?: boolean;
}

/**
 * Opens a match's public page: the room, the score and the chat, live. For the match's own players
 * (until it's over) it opens their room page instead, "Maç odası", lit like a live match: it's
 * theirs. Not shown on admin pages to those who can open the referee page instead (referees and
 * hosts), which shows all of it.
 */
export function MatchPageLink({
  matchId,
  live = false,
  variant,
  teamIds = [],
  concluded = false,
}: Props) {
  const { user } = useAuth();
  const admin = isAdminPath(useLocation().pathname);
  if (admin && hasRole(user, "referee")) return null;
  const own = !admin && isOwnMatch(user?.team_id, teamIds, concluded);
  const label = own ? "Maç odası" : live ? "Canlı takip et" : "Maç sayfası";
  const lit = own || live;
  const classes = {
    card: "match-page-link is-card",
    row: "icon-btn icon-btn-outlined match-page-link is-row",
    text: "btn btn-secondary btn-sm match-page-link is-text",
  }[variant];
  return (
    <Link
      to={own ? roomPagePath(matchId) : matchPagePath(matchId)}
      className={lit ? `${classes} is-live` : classes}
      aria-label={variant === "text" ? undefined : label}
      title={label}
      // Inside a grid card, a list row or a calendar chip, so following it doesn't also select the
      // match, and the grid neither starts dragging the card nor moves it with the arrow keys.
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
      draggable={false}
    >
      <Icon name={own ? "swords" : "monitor"} size={variant === "row" ? 16 : 14} />
      {variant === "text" && <span>{label}</span>}
    </Link>
  );
}
