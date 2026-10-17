import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";
import { matchPagePath } from "../lib/bracket";
import { isAdminPath } from "../pages/admin/tabs";
import { Icon } from "./Icon";
import "./MatchPageLink.css";

interface Props {
  matchId: string;
  /** Being played right now: the link says so and is lit. */
  live?: boolean;
  /**
   * "card" is a bare icon for the grid's cards and the calendar's chips, "row" an icon button for
   * list rows, "text" a labelled link for headers and lobby cards.
   */
  variant: "card" | "row" | "text";
}

/**
 * Opens a match's public page: the room, the score and the chat, live. Not shown on admin pages
 * to those who can open the referee page instead (referees and hosts), which shows all of it.
 */
export function MatchPageLink({ matchId, live = false, variant }: Props) {
  const { user } = useAuth();
  const admin = isAdminPath(useLocation().pathname);
  if (admin && hasRole(user, "referee")) return null;
  const label = live ? "Canlı takip et" : "Maç sayfası";
  const classes = {
    card: "match-page-link is-card",
    row: "icon-btn icon-btn-outlined match-page-link is-row",
    text: "match-page-link is-text",
  }[variant];
  return (
    <Link
      to={matchPagePath(matchId)}
      className={live ? `${classes} is-live` : classes}
      aria-label={variant === "text" ? undefined : label}
      title={label}
      // Inside a grid card, a list row or a calendar chip, so following it doesn't also select the
      // match, and the grid neither starts dragging the card nor moves it with the arrow keys.
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      onKeyDown={(e) => e.stopPropagation()}
      draggable={false}
    >
      <Icon name="monitor" size={variant === "card" ? 14 : 16} />
      {variant === "text" && <span>{label}</span>}
    </Link>
  );
}
