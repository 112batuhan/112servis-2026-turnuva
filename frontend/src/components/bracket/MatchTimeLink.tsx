import { MatchTime } from "./MatchTime";
import "./MatchTimeLink.css";

interface Props {
  matchId: string;
  startsAt: string;
  countdown?: boolean;
  onShowInCalendar: (id: string) => void;
}

/** A scheduled match's time that opens the calendar on it. */
export function MatchTimeLink({ matchId, startsAt, countdown, onShowInCalendar }: Props) {
  return (
    <button
      type="button"
      className="match-time-link"
      onClick={(e) => {
        // Inside a grid card, so the card isn't selected as well.
        e.stopPropagation();
        onShowInCalendar(matchId);
      }}
      // Enter on it would otherwise also select the card, and arrows would move it.
      onKeyDown={(e) => e.stopPropagation()}
    >
      <MatchTime startsAt={startsAt} countdown={countdown} hint="Takvimde göster" />
    </button>
  );
}
