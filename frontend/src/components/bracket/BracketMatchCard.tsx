import { API_URL } from "../../api/client";
import type { BracketMatch, Team } from "../../api/types";
import { matchCode, sourceLabel, winningSlot, type Source } from "../../lib/bracket";
import { formatShortMatchTime } from "../../lib/format";
import { Icon } from "../Icon";
import "./BracketMatchCard.css";

interface Props {
  match: BracketMatch;
  teams: Map<string, Team>;
  /** What feeds each slot, shown while the slot is empty. */
  sources: [Source | null, Source | null];
  myTeamId?: string | null;
}

/** A match as the grid shows it. Sized by its grid cell. */
export function BracketMatchCard({ match, teams, sources, myTeamId }: Props) {
  const winner = winningSlot(match);
  const slots = [
    { id: match.team1_id, score: match.team1_score, source: sources[0] },
    { id: match.team2_id, score: match.team2_score, source: sources[1] },
  ];

  return (
    <div className="bm-card">
      <div className="bm-card-header">
        <span className={`bm-card-code is-${match.side}`}>{matchCode(match)}</span>
        <span className="bm-card-time">
          {match.starts_at ? formatShortMatchTime(match.starts_at) : <span className="muted">Zaman yok</span>}
        </span>
        {match.streamer && (
          <span className="bm-card-icon" title={`Yayıncı: ${match.streamer.username}`}>
            <Icon name="radio" size={14} />
          </span>
        )}
        {match.concluded_at && (
          <span className="bm-card-icon is-done" title="Tamamlandı">
            <Icon name="check" size={14} />
          </span>
        )}
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
            <span className="bm-card-banner">
              {team?.banner_url && <img src={`${API_URL}${team.banner_url}`} alt="" loading="lazy" />}
            </span>
            <span className={team ? "bm-card-name" : "bm-card-name is-empty"}>
              {team ? team.name : slot.source ? sourceLabel(slot.source) : "Belirlenmedi"}
            </span>
            <span className="bm-card-score">{slot.score ?? ""}</span>
          </div>
        );
      })}
    </div>
  );
}
