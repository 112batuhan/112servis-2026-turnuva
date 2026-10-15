import { Link } from "react-router-dom";
import type { BracketMatch, Team } from "../api/types";
import { matchCode, sourceLabel, winningSlot, type Source } from "../lib/bracket";
import { formatShortMatchTime } from "../lib/format";
import { Countdown } from "./Countdown";
import { TeamBanner } from "./bracket/TeamBanner";
import "./FixtureRow.css";

interface Props {
  match: BracketMatch;
  /** Where the row leads, usually the match on the bracket. */
  to: string;
  team1: Team | undefined;
  team2: Team | undefined;
  /** What feeds each empty slot ("W3 kazananı"). */
  sources?: [Source | null, Source | null];
  /** Marks the viewer's own team in the row. */
  ownTeamId?: string | null;
}

/**
 * One match as a line of a fixture sheet: the time, the red team, the score (or "vs"), the blue
 * team, then the match code and stage. Team 1 plays red and team 2 blue, as in the osu! room.
 */
export function FixtureRow({ match, to, team1, team2, sources, ownTeamId }: Props) {
  const winner = winningSlot(match);
  const started = match.starts_at !== null && new Date(match.starts_at).getTime() <= Date.now();
  const live = !match.concluded_at && started && match.rooms.length > 0;

  const side = (slot: 1 | 2) => {
    const team = slot === 1 ? team1 : team2;
    const source = sources?.[slot - 1] ?? null;
    const classes = [
      "fixture-team",
      `is-slot-${slot}`,
      winner !== null && winner !== slot ? "is-loser" : "",
      team && team.id === ownTeamId ? "is-own" : "",
    ]
      .filter(Boolean)
      .join(" ");
    return (
      <span className={classes}>
        <TeamBanner team={team} />
        <span className={team ? "fixture-team-name" : "fixture-team-name is-empty"}>
          {team ? team.name : source ? sourceLabel(source) : "Belirlenmedi"}
        </span>
      </span>
    );
  };

  return (
    <Link to={to} className={live ? "fixture-row is-live" : "fixture-row"}>
      <span className="fixture-when">
        {live ? (
          <span className="fixture-live">Canlı</span>
        ) : match.starts_at ? (
          <>
            <span className="fixture-time">{formatShortMatchTime(match.starts_at)}</span>
            {!match.concluded_at && !started && (
              <Countdown to={match.starts_at} suffix="kaldı" className="fixture-countdown" />
            )}
          </>
        ) : (
          <span className="fixture-time">Saat yok</span>
        )}
      </span>
      {side(1)}
      <span className="fixture-score" aria-label={match.concluded_at ? "Skor" : undefined}>
        {match.concluded_at ? (
          <>
            <span className={winner === 1 ? "is-winner" : undefined}>{match.team1_score ?? "–"}</span>
            <span className={winner === 2 ? "is-winner" : undefined}>{match.team2_score ?? "–"}</span>
          </>
        ) : (
          <span className="fixture-vs">vs</span>
        )}
      </span>
      {side(2)}
      <span className="fixture-meta">
        <span className={`fixture-code is-${match.side}`}>{matchCode(match)}</span>
        {match.stage && <span className="fixture-stage">{match.stage.name}</span>}
      </span>
    </Link>
  );
}
