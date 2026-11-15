import { Link } from "react-router-dom";
import { Icon } from "../Icon";
import "./RefereeRoomLink.css";

interface Props {
  matchId: string;
  /** Being played right now: the button is lit, like the match page's. */
  live?: boolean;
  /**
   * "card" is a bare icon for the grid's cards and the calendar's chips; "text" a labelled small
   * button for the admin list's rows, in the match page link's place.
   */
  variant: "card" | "text";
}

/** Opens a match's referee room. */
export function RefereeRoomLink({ matchId, live = false, variant }: Props) {
  const classes =
    variant === "text" ? "btn btn-secondary btn-sm referee-room-link is-text" : "referee-room-link";
  return (
    <Link
      to={`/admin/referee/${matchId}`}
      className={live ? `${classes} is-live` : classes}
      aria-label={variant === "text" ? undefined : "Hakem odası"}
      title="Hakem odası"
      // Inside a grid card, a list row or a calendar chip, so following it doesn't also select the
      // match, and the grid neither starts dragging the card nor moves it with the arrow keys.
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      draggable={false}
      onKeyDown={(e) => e.stopPropagation()}
    >
      <Icon name="gavel" size={14} />
      {variant === "text" && <span>Hakem odası</span>}
    </Link>
  );
}
