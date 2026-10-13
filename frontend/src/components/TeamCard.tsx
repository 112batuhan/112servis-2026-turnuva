import type { ReactNode } from "react";
import type { Player, Team } from "../api/types";
import { formatTeamValue, TEAM_NAME_MAX } from "../lib/format";
import { InlineEdit } from "./InlineEdit";
import { PlayerChip } from "./PlayerChip";
import "./TeamCard.css";

interface Props<P extends Player> {
  team: Team<P>;
  /** Extra content under each player, e.g. their Discord in the admin list. */
  renderPlayerExtra?: (player: P) => ReactNode;
  /** Makes the name editable. */
  onRename?: (name: string) => void;
  actions?: ReactNode;
}

export function TeamCard<P extends Player>({ team, renderPlayerExtra, onRename, actions }: Props<P>) {
  return (
    <article className="team-card">
      <h3 className="team-card-name">
        {onRename ? (
          <InlineEdit value={team.name} onSave={onRename} label="Rename team" maxLength={TEAM_NAME_MAX} />
        ) : (
          team.name
        )}
      </h3>
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
