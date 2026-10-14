import { Link } from "react-router-dom";
import { Icon } from "../Icon";
import "./RefereeRoomLink.css";

interface Props {
  matchId: string;
  /** "card" is a bare icon for the grid's cards and the calendar's chips; "row" is an icon button. */
  variant: "card" | "row";
}

/** Opens a match's referee room. */
export function RefereeRoomLink({ matchId, variant }: Props) {
  return (
    <Link
      to={`/admin/referee/${matchId}`}
      className={variant === "row" ? "icon-btn icon-btn-outlined" : "referee-room-link"}
      aria-label="Hakem odası"
      title="Hakem odası"
      // Inside a grid card or a calendar chip, so following it doesn't also select the match,
      // and the grid neither starts dragging the card nor moves it with the arrow keys.
      onClick={(e) => e.stopPropagation()}
      onPointerDown={(e) => e.stopPropagation()}
      draggable={false}
      onKeyDown={(e) => e.stopPropagation()}
    >
      <Icon name="gavel" size={variant === "row" ? 16 : 14} />
    </Link>
  );
}
