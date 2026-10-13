import type { ReactNode } from "react";
import { API_URL } from "../api/client";
import type { Player, Team } from "../api/types";
import { RANK_WARNING_LABELS } from "../lib/eligibility";
import { formatTeamValue, TEAM_NAME_MAX } from "../lib/format";
import { teamAnchorId } from "../lib/teamLink";
import { InlineEdit } from "./InlineEdit";
import { PlayerChip } from "./PlayerChip";
import "./TeamCard.css";

interface Props<P extends Player> {
  team: Team<P>;
  /** Place in the list, which is ordered by team value. */
  position?: number;
  /** Extra content under each player, e.g. their Discord in the admin list. */
  renderPlayerExtra?: (player: P) => ReactNode;
  /** Makes the name editable. */
  onRename?: (name: string) => void;
  actions?: ReactNode;
  /** Marks the team someone was linked to. */
  highlight?: boolean;
}

export function TeamCard<P extends Player>({
  team,
  position,
  renderPlayerExtra,
  onRename,
  actions,
  highlight,
}: Props<P>) {
  const classes = ["team-card", team.rank_warning && "is-warning", highlight && "is-target"].filter(Boolean).join(" ");
  return (
    <article id={teamAnchorId(team.id)} className={classes}>
      <div className="team-card-banner">
        {team.banner_url && <img src={`${API_URL}${team.banner_url}`} alt="" />}
        <div className="team-card-title">
          {position !== undefined && <span className="team-card-position">#{position}</span>}
          <h3 className="team-card-name">
            {onRename ? (
              <InlineEdit value={team.name} onSave={onRename} label="Takımı yeniden adlandır" maxLength={TEAM_NAME_MAX} />
            ) : (
              team.name
            )}
          </h3>
        </div>
      </div>
      <div className="team-card-body">
        <div className="team-card-value">
          <span className="team-card-value-label">Hesaplanan rank</span>
          <span className="team-card-value-number">{formatTeamValue(team.team_value)}</span>
          {team.rank_warning && <span className="team-card-warning">{RANK_WARNING_LABELS[team.rank_warning]}</span>}
        </div>
        <ul className="team-card-players">
          {team.players.map((p) => (
            <li key={p.id}>
              <PlayerChip player={p} />
              {renderPlayerExtra && <div className="team-card-extra">{renderPlayerExtra(p)}</div>}
            </li>
          ))}
        </ul>
      </div>
      {actions && <div className="team-card-actions">{actions}</div>}
    </article>
  );
}
