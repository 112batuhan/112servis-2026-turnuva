import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { isAdminPath } from "../lib/adminTabs";
import type { BracketMatch, Team } from "../api/types";
import { stageLabel } from "../lib/bestOf";
import {
  hasMatchPage,
  isLive,
  isOwnMatch,
  matchCode,
  roomPagePath,
  scoreHalves,
  sourceLabel,
  winningSlot,
  type Source,
} from "../lib/bracket";
import { formatShortMatchTime } from "../lib/format";
import { Countdown } from "./Countdown";
import { Icon } from "./Icon";
import { LiveMark } from "./LiveMark";
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
 * team, then the match code and stage. Team 1 plays blue and team 2 red, as in the osu! room.
 * A potential match (`potential_of`) reads as one that may never be played: "olası" by its code
 * and the score's fill given way to a dashed outline.
 */
export function FixtureRow({ match, to, team1, team2, sources, ownTeamId }: Props) {
  const { user } = useAuth();
  const admin = isAdminPath(useLocation().pathname);
  const winner = winningSlot(match);
  const started = match.starts_at !== null && new Date(match.starts_at).getTime() <= Date.now();
  const live = isLive(match);
  // A concluded match's result, or the running score of a live one as its referee saves it.
  const halves = match.concluded_at || live ? scoreHalves(match) : null;
  // While it's live, whoever is ahead reads like a winner; level, neither does.
  const leading =
    live &&
    match.team1_score !== null &&
    match.team2_score !== null &&
    match.team1_score !== match.team2_score
      ? match.team1_score > match.team2_score
        ? 1
        : 2
      : null;
  const ahead = winner ?? leading;
  const potential = match.potential_of !== undefined;
  // The viewer's own match leads to their room page, and says so where the stage would be.
  const room =
    !admin &&
    !potential &&
    hasMatchPage(match) &&
    isOwnMatch(user?.team_id, [match.team1_id, match.team2_id], match.concluded_at !== null);

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
    <Link
      to={room ? roomPagePath(match.id) : to}
      className={["fixture-row", live && "is-live", potential && "is-potential"]
        .filter(Boolean)
        .join(" ")}
      title={potential ? "İki takım da bu maça gelirse oynanır" : undefined}
    >
      <span className="fixture-when">
        {live ? (
          <LiveMark />
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
      <span
        className={live ? "fixture-score is-live" : "fixture-score"}
        aria-label={
          halves
            ? match.forfeit_slot && match.concluded_at
              ? "Hükmen sonuç"
              : live
                ? "Canlı skor"
                : "Skor"
            : undefined
        }
      >
        {halves ? (
          <>
            {/* Keyed by the figure, so a half that changes while live is new and flashes once. */}
            <span key={`1:${halves[0]}`} className={ahead === 1 ? "is-winner" : undefined}>
              {halves[0]}
            </span>
            <span key={`2:${halves[1]}`} className={ahead === 2 ? "is-winner" : undefined}>
              {halves[1]}
            </span>
          </>
        ) : match.concluded_at ? (
          <>
            <span>–</span>
            <span>–</span>
          </>
        ) : (
          <span className="fixture-vs">vs</span>
        )}
      </span>
      {side(2)}
      <span className="fixture-meta">
        <span className="fixture-code-line">
          {potential && <span className="fixture-potential">olası</span>}
          <span className={`fixture-code is-${match.side}`}>{matchCode(match)}</span>
        </span>
        {room ? (
          <span className="fixture-watch is-room">
            <Icon name="swords" size={13} />
            Maç odası
          </span>
        ) : live ? (
          // A live row opens the match's page: say so where the stage would be.
          <span className="fixture-watch">
            <Icon name="monitor" size={13} />
            Canlı takip et
          </span>
        ) : (
          match.stage && <span className="fixture-stage">{stageLabel(match.stage)}</span>
        )}
      </span>
    </Link>
  );
}
