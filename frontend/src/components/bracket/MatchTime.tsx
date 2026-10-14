import { useReducer } from "react";
import { formatShortMatchTime } from "../../lib/format";
import { Countdown } from "../Countdown";

interface Props {
  startsAt: string;
  /** Shows the time left instead of the date, until the match starts. */
  countdown?: boolean;
  /** Extra tooltip text, after the date when the countdown hides it. */
  hint?: string;
}

/** A match's start: its date, or a live countdown to it. A time already reached shows the date. */
export function MatchTime({ startsAt, countdown, hint }: Props) {
  // Re-renders when the countdown reaches zero, so the date takes its place.
  const [, rerender] = useReducer((n: number) => n + 1, 0);
  const date = formatShortMatchTime(startsAt);
  if (!countdown || new Date(startsAt).getTime() <= Date.now()) {
    return <span title={hint}>{date}</span>;
  }
  return (
    <span title={hint ? `${date} · ${hint}` : date}>
      <Countdown to={startsAt} onElapsed={rerender} />
    </span>
  );
}
