import { useId, useState, type ReactNode } from "react";
import { API_URL } from "../api/client";
import type { Player, QualifierPlace, Team } from "../api/types";
import { RANK_WARNING_LABELS } from "../lib/eligibility";
import { formatTeamValue, TEAM_NAME_MAX } from "../lib/format";
import { teamAnchorId } from "../lib/teamLink";
import { Icon } from "./Icon";
import { InlineEdit } from "./InlineEdit";
import { PlayerChip } from "./PlayerChip";
import { QualifierFact } from "./TeamCard";
import { TeamMatches } from "./TeamMatches";
import "./TeamRow.css";

/** The row list's frame: a line of column names over the rows, which line up through a subgrid. */
export function TeamRows({
  numbered,
  qualifier,
  children,
}: {
  numbered?: boolean;
  /** Adds the qualifier place column; pass `qualifier` to every row then. */
  qualifier?: boolean;
  children: ReactNode;
}) {
  const classes = ["team-lines", numbered && "is-numbered", qualifier && "has-qualifier"]
    .filter(Boolean)
    .join(" ");
  return (
    <div className="team-lines-frame">
      <div className={classes}>
        <div className="team-lines-head" aria-hidden="true">
          {numbered && <span>#</span>}
          <span className="team-lines-head-team">Takım</span>
          <span>Oyuncular</span>
          {qualifier && <span className="team-lines-head-value">Eleme</span>}
          <span className="team-lines-head-value">Takım değeri</span>
          <span />
        </div>
        <ol className="team-lines-list">{children}</ol>
      </div>
    </div>
  );
}

interface Props<P extends Player> {
  team: Team<P>;
  /** Place in the list, which is ordered by team value. Pass it to every row of a `numbered` list. */
  position?: number;
  /** Extra content under each player, e.g. their Discord in the admin list. */
  renderPlayerExtra?: (player: P) => ReactNode;
  /** Makes the name editable. */
  onRename?: (name: string) => void;
  actions?: ReactNode;
  /** Marks the team someone was linked to. */
  highlight?: boolean;
  /**
   * A warning the page adds beside the rank one, e.g. the admin list's banned players. It flags the
   * team like a rank warning does, but leaves the value alone: it isn't what's wrong.
   */
  flag?: string;
  /** Its qualifier place, in a `qualifier` list (null: none yet). Left out, the column isn't there. */
  qualifier?: QualifierPlace | null;
}

/** One team as a line of the row list (`TeamRows`): the same facts as its `TeamCard`, read across. */
export function TeamRow<P extends Player>({
  team,
  position,
  renderPlayerExtra,
  onRename,
  actions,
  highlight,
  flag,
  qualifier,
}: Props<P>) {
  const [matchesOpen, setMatchesOpen] = useState(false);
  const matchesId = useId();
  const classes = [
    "team-line",
    team.rank_warning && "is-warning",
    flag && "is-flagged",
    highlight && "is-target",
    matchesOpen && "is-open",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <li id={teamAnchorId(team.id)} className={classes}>
      <div className="team-line-main">
        {position !== undefined && <span className="team-line-position num">{position}</span>}
        <div
          className={team.banner_url ? "team-line-banner" : "team-line-banner is-empty"}
          aria-hidden="true"
        >
          {team.banner_url ? (
            <img src={`${API_URL}${team.banner_url}`} alt="" loading="lazy" />
          ) : (
            <span className="team-line-monogram">
              {Array.from(team.name)[0]?.toLocaleUpperCase("tr-TR")}
            </span>
          )}
        </div>
        <div className="team-line-title">
          <h3 className="team-line-name">
            {onRename ? (
              <InlineEdit
                value={team.name}
                onSave={onRename}
                label="Takımı yeniden adlandır"
                maxLength={TEAM_NAME_MAX}
              />
            ) : (
              team.name
            )}
          </h3>
          {team.rank_warning && (
            <span className="team-line-warning">{RANK_WARNING_LABELS[team.rank_warning]}</span>
          )}
          {flag && <span className="team-line-warning">{flag}</span>}
        </div>
        <ul className="team-line-players">
          {team.players.map((p) => (
            <li key={p.id}>
              <PlayerChip player={p} />
              {renderPlayerExtra && <div className="team-line-extra">{renderPlayerExtra(p)}</div>}
            </li>
          ))}
        </ul>
        {qualifier !== undefined && (
          <span className="team-line-qualifier">
            {qualifier ? (
              <QualifierFact place={qualifier} compact />
            ) : (
              <span className="team-line-none">—</span>
            )}
          </span>
        )}
        <span
          className="team-line-value num"
          aria-label={`Takım değeri ${formatTeamValue(team.team_value)}`}
        >
          {formatTeamValue(team.team_value)}
        </span>
        <div className="team-line-tools">
          <button
            type="button"
            className="team-line-matches-toggle"
            aria-expanded={matchesOpen}
            aria-controls={matchesId}
            onClick={() => setMatchesOpen((open) => !open)}
          >
            <Icon name="bracket" size={14} className="team-line-toggle-icon" />
            <span className="team-line-toggle-label">
              Maçlar{actions && <span className="team-line-toggle-more"> ve işlemler</span>}
            </span>
            <Icon name="chevronDown" size={14} className="team-line-chevron" />
          </button>
          {actions && <span className="team-line-actions">{actions}</span>}
        </div>
      </div>
      {matchesOpen && (
        <div
          id={matchesId}
          className="team-line-matches"
          role="region"
          aria-label={`${team.name} maçları`}
        >
          {/* Where the row is too narrow to carry them, the actions wait here instead. */}
          {actions && <div className="team-line-panel-actions">{actions}</div>}
          <TeamMatches teamId={team.id} />
        </div>
      )}
    </li>
  );
}
