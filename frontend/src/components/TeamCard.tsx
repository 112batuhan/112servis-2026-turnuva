import type { ReactNode } from "react";
import type { Player, Team } from "../api/types";
import { formatTeamValue } from "../lib/format";
import { PlayerChip } from "./PlayerChip";
import "./TeamCard.css";

interface Props<P extends Player> {
  team: Team<P>;
  /** Extra content under each player, e.g. their Discord in the admin list. */
  renderPlayerExtra?: (player: P) => ReactNode;
  actions?: ReactNode;
}

export function TeamCard<P extends Player>({ team, renderPlayerExtra, actions }: Props<P>) {
  return (
    <article className="team-card">
      <ul className="team-card-players">
        {team.players.map((p) => (
          <li key={p.id}>
            <PlayerChip player={p} />
            {renderPlayerExtra && <div className="team-card-extra">{renderPlayerExtra(p)}</div>}
          </li>
        ))}
      </ul>
      <div className="team-card-footer">
        <span className="badge" title="Team value">
          {formatTeamValue(team.team_value)}
        </span>
        {actions}
      </div>
    </article>
  );
}
