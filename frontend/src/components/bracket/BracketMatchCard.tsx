import type { BracketMatch, RefereeViewer, Team } from "../../api/types";
import {
  hasMatchPage,
  isLive,
  isUndecided,
  matchCode,
  scoreHalves,
  sourceLabel,
  UNDECIDED_TIME,
  winningSlot,
  type Source,
} from "../../lib/bracket";
import { stageLabel } from "../../lib/bestOf";
import { Icon } from "../Icon";
import { LiveMark } from "../LiveMark";
import { MatchPageLink } from "../MatchPageLink";
import { TeamPopover } from "../TeamPopover";
import { MatchTime } from "./MatchTime";
import { MatchTimeLink } from "./MatchTimeLink";
import { RefereeRoomLink } from "./RefereeRoomLink";
import { RefereeWatchers } from "./RefereeWatchers";
import { TeamBanner } from "./TeamBanner";
import "./BracketMatchCard.css";

interface Props {
  match: BracketMatch;
  teams: Map<string, Team>;
  /** What feeds each slot, shown while the slot is empty. */
  sources: [Source | null, Source | null];
  myTeamId?: string | null;
  /** Makes the time a button that opens the calendar on this match. */
  onShowInCalendar?: (id: string) => void;
  /** Shows the time left until the match instead of its date. */
  countdown?: boolean;
  /** Adds a link to the match's referee room. */
  showRefereeRoom?: boolean;
  /** Who has the match's referee page open (admin only). */
  watchers?: RefereeViewer[];
}

/** A match as the grid shows it. Sized by its grid cell. */
export function BracketMatchCard({
  match,
  teams,
  sources,
  myTeamId,
  onShowInCalendar,
  countdown,
  showRefereeRoom,
  watchers,
}: Props) {
  const winner = winningSlot(match);
  const live = isLive(match);
  const halves = scoreHalves(match);
  const slots = [
    { id: match.team1_id, score: halves?.[0], source: sources[0] },
    { id: match.team2_id, score: halves?.[1], source: sources[1] },
  ];

  return (
    <div className={live ? "bm-card is-live" : "bm-card"}>
      <div className="bm-card-header">
        <span className={`bm-card-code is-${match.side}`}>{matchCode(match)}</span>
        {match.stage?.abbreviation && (
          // Its stage, short, so the card says it wherever the board is scrolled; hovered, in full.
          <abbr className="bm-card-stage" title={stageLabel(match.stage)}>
            {match.stage.abbreviation}
          </abbr>
        )}
        {live && <LiveMark compact />}
        <span className="bm-card-time">
          {isUndecided(match) ? (
            // Its time is whichever its potential matches' teams agree on; the list and the calendar show them.
            <span className="bm-card-undecided" title={`${match.potentials.length} olası eşleşme`}>
              {UNDECIDED_TIME}
            </span>
          ) : !match.starts_at ? (
            <span className="muted">Zaman yok</span>
          ) : onShowInCalendar ? (
            <MatchTimeLink
              matchId={match.id}
              startsAt={match.starts_at}
              countdown={countdown}
              onShowInCalendar={onShowInCalendar}
            />
          ) : (
            <MatchTime startsAt={match.starts_at} countdown={countdown} />
          )}
        </span>
        {match.streamer && (
          <span className="bm-card-icon" title={`Yayıncı: ${match.streamer.username}`}>
            <Icon name="radio" size={14} />
          </span>
        )}
        {match.concluded_at && (
          <span
            className="bm-card-icon is-done"
            title={match.forfeit_slot !== null ? "Hükmen sonuçlandı" : "Tamamlandı"}
          >
            <Icon name="check" size={14} />
          </span>
        )}
        <RefereeWatchers viewers={watchers} />
        {hasMatchPage(match) && (
          <MatchPageLink
            matchId={match.id}
            live={live}
            variant="card"
            teamIds={[match.team1_id, match.team2_id]}
            concluded={match.concluded_at !== null}
          />
        )}
        {showRefereeRoom && <RefereeRoomLink matchId={match.id} variant="card" />}
      </div>
      {slots.map((slot, i) => {
        const team = slot.id ? teams.get(slot.id) : undefined;
        const classes = [
          "bm-card-team",
          winner === i + 1 && "is-winner",
          winner !== null && winner !== i + 1 && "is-loser",
          myTeamId && slot.id === myTeamId && "is-mine",
        ]
          .filter(Boolean)
          .join(" ");
        return (
          <div key={i} className={classes}>
            {team ? (
              <TeamPopover team={team}>
                <TeamBanner team={team} />
                <span className="bm-card-name">{team.name}</span>
              </TeamPopover>
            ) : (
              <>
                <TeamBanner />
                <span className="bm-card-name is-empty">
                  {slot.source ? sourceLabel(slot.source) : "Belirlenmedi"}
                </span>
              </>
            )}
            <span className="bm-card-score">{slot.score ?? ""}</span>
          </div>
        );
      })}
    </div>
  );
}
