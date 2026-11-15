import { useEffect, useRef, useState, type ReactNode } from "react";
import { API_URL } from "../api/client";
import type { Player, Team } from "../api/types";
import { RANK_WARNING_LABELS } from "../lib/eligibility";
import { formatTeamValue, TEAM_NAME_MAX } from "../lib/format";
import { teamAnchorId } from "../lib/teamLink";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";
import { InlineEdit } from "./InlineEdit";
import { PlayerChip } from "./PlayerChip";
import { TeamMatches } from "./TeamMatches";
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
  /** Adds a button that shows the team's next match and match history over the card. */
  showMatches?: boolean;
  /** Shows the team value; off where the page already shows it, like /team's meter. */
  showValue?: boolean;
  /**
   * A warning the page adds beside the rank one, e.g. the admin list's banned players. It flags the
   * team like a rank warning does, but leaves the value alone: it isn't what's wrong.
   */
  flag?: string;
}

export function TeamCard<P extends Player>({
  team,
  position,
  renderPlayerExtra,
  onRename,
  actions,
  highlight,
  showMatches = true,
  showValue = true,
  flag,
}: Props<P>) {
  const [matchesOpen, setMatchesOpen] = useState(false);
  const overlayRef = useRef<HTMLDivElement>(null);
  const toggleRef = useRef<HTMLButtonElement>(null);
  const classes = [
    "team-card",
    team.rank_warning && "is-warning",
    flag && "is-flagged",
    highlight && "is-target",
    matchesOpen && "is-showing-matches",
  ]
    .filter(Boolean)
    .join(" ");

  // Focus follows the overlay in and back out to the button that opened it.
  useEffect(() => {
    if (matchesOpen) overlayRef.current?.focus({ preventScroll: true });
  }, [matchesOpen]);

  const closeMatches = () => {
    setMatchesOpen(false);
    toggleRef.current?.focus({ preventScroll: true });
  };

  return (
    <article id={teamAnchorId(team.id)} className={classes}>
      <div
        className={team.banner_url ? "team-card-banner" : "team-card-banner is-empty"}
        aria-hidden={matchesOpen || undefined}
      >
        {team.banner_url ? (
          <img src={`${API_URL}${team.banner_url}`} alt="" />
        ) : (
          // No banner yet: the team's initial, set large, holds the space instead of an empty box.
          <span className="team-card-monogram" aria-hidden="true">
            {Array.from(team.name)[0]?.toLocaleUpperCase("tr-TR")}
          </span>
        )}
        {showMatches && (
          <button
            ref={toggleRef}
            type="button"
            className="team-card-matches-toggle"
            aria-expanded={matchesOpen}
            tabIndex={matchesOpen ? -1 : undefined}
            onClick={() => setMatchesOpen(true)}
          >
            <Icon name="bracket" size={14} />
            Maçlar
          </button>
        )}
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
      <div className={showValue ? "team-card-body" : "team-card-body is-players-only"} aria-hidden={matchesOpen || undefined}>
        {showValue && (
          <div className="team-card-value">
            <span className="team-card-value-label">Takım değeri</span>
            <span className="team-card-value-number num">{formatTeamValue(team.team_value)}</span>
            {team.rank_warning && <span className="team-card-warning">{RANK_WARNING_LABELS[team.rank_warning]}</span>}
            {flag && <span className="team-card-warning">{flag}</span>}
          </div>
        )}
        <ul className="team-card-players">
          {team.players.map((p) => (
            <li key={p.id}>
              <PlayerChip player={p} />
              {renderPlayerExtra && <div className="team-card-extra">{renderPlayerExtra(p)}</div>}
            </li>
          ))}
        </ul>
      </div>
      {actions && (
        <div className="team-card-actions" aria-hidden={matchesOpen || undefined}>
          {actions}
        </div>
      )}
      {matchesOpen && (
        <div
          ref={overlayRef}
          className="team-card-matches"
          role="region"
          aria-label={`${team.name} maçları`}
          tabIndex={-1}
          onKeyDown={(e) => {
            if (e.key !== "Escape") return;
            // Only the overlay closes, not a popover the card is in.
            e.stopPropagation();
            closeMatches();
          }}
        >
          <div className="team-card-matches-header">
            <h3>{team.name}</h3>
            <IconButton icon="close" label="Maçları kapat" onClick={closeMatches} />
          </div>
          <TeamMatches teamId={team.id} />
        </div>
      )}
    </article>
  );
}
