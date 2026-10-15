import { useMemo, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import type { BracketMatch, BracketSide, Team } from "../../api/types";
import {
  groupByStage,
  incomingSources,
  matchCode,
  SIDE_LABELS,
  sourceLabel,
  winningSlot,
  type Source,
} from "../../lib/bracket";
import { readString, writeString } from "../../lib/storage";
import { useStoredSet } from "../../lib/useExpandedSet";
import { Icon } from "../Icon";
import { RoomLinks } from "../RoomLinks";
import { TeamPopover } from "../TeamPopover";
import { MatchTime } from "./MatchTime";
import { MatchTimeLink } from "./MatchTimeLink";
import { TeamBanner } from "./TeamBanner";
import { DiscordMention } from "../DiscordMention";
import "./BracketList.css";
// The side filter looks like the view tabs.
import "./BracketViewTabs.css";

type SideFilter = BracketSide | "all";

const SIDE_FILTERS: { value: SideFilter; label: string }[] = [
  { value: "all", label: "Hepsi" },
  { value: "winners", label: SIDE_LABELS.winners },
  { value: "losers", label: SIDE_LABELS.losers },
];

const SIDE_FILTER_KEY = "bracket-list-side";

/** Clicks on these keep their own meaning instead of selecting the row's match. */
const ROW_CONTROLS = "a, button, input, select, textarea, label";

interface Props<M extends BracketMatch> {
  matches: M[];
  teams: Map<string, Team>;
  myTeamId?: string | null;
  viewerId?: string;
  selectedId?: string | null;
  onSelect?: (id: string) => void;
  /** Replaces the referee's name and Discord, e.g. with controls to change who it is. */
  renderReferee?: (match: M) => ReactNode;
  /** Replaces the streamer's name, likewise. */
  renderStreamer?: (match: M) => ReactNode;
  /** Controls at the end of each row. */
  renderActions?: (match: M) => ReactNode;
  /** Makes each time a button that opens the calendar on that match. */
  onShowInCalendar?: (id: string) => void;
  /** Shows the time left until each match instead of its date. */
  countdown?: boolean;
  /** The ids of the matches the search finds, or null when not searching. */
  hits?: Set<string> | null;
}

/**
 * Every match grouped by stage, in stage order; each stage in time order, unscheduled ones last.
 * Stages can be collapsed and the list narrowed to one side (both remembered per browser) or to the search's hits.
 */
export function BracketList<M extends BracketMatch>({
  matches,
  teams,
  myTeamId,
  viewerId,
  selectedId,
  onSelect,
  renderReferee,
  renderStreamer,
  renderActions,
  onShowInCalendar,
  countdown,
  hits,
}: Props<M>) {
  const [side, setSide] = useState<SideFilter>(() => {
    const saved = readString(SIDE_FILTER_KEY);
    return SIDE_FILTERS.some((f) => f.value === saved) ? (saved as SideFilter) : "all";
  });
  const selectSide = (next: SideFilter) => {
    setSide(next);
    writeString(SIDE_FILTER_KEY, next);
  };
  const shown = useMemo(
    () => matches.filter((m) => (side === "all" || m.side === side) && (!hits || hits.has(m.id))),
    [matches, side, hits],
  );
  const groups = useMemo(() => groupByStage(shown), [shown]);
  // From every match, so a slot fed from the other side still names its source.
  const sources = useMemo(() => incomingSources(matches), [matches]);
  const { has: isCollapsed, toggle } = useStoredSet("bracket-list-collapsed");
  const now = new Date().toISOString();

  return (
    <div className="bracket-list-wrap">
      <div className="bracket-list-toolbar">
        <div className="bracket-view-tabs" role="group" aria-label="Taraf">
          {SIDE_FILTERS.map((f) => (
            <button
              key={f.value}
              type="button"
              className="bracket-view-tab"
              aria-pressed={side === f.value}
              onClick={() => selectSide(f.value)}
            >
              {f.label}
            </button>
          ))}
        </div>
      </div>
      {groups.length === 0 ? (
        <p className="status">{hits ? "Aramayla eşleşen maç yok." : "Bu tarafta maç yok."}</p>
      ) : (
        <div className="bracket-list-scroll">
          <table className="bracket-list">
            <thead>
              <tr>
                <th scope="col">Maç</th>
                <th scope="col">Zaman</th>
                <th scope="col">Eşleşme</th>
                <th scope="col">Hakem</th>
                <th scope="col">Yayıncı</th>
                <th scope="col">Durum</th>
                {renderActions && (
                  <th scope="col">
                    <span className="sr-only">İşlemler</span>
                  </th>
                )}
              </tr>
            </thead>
            {groups.map((group) => {
              const key = group.stage?.id ?? "none";
              const collapsed = isCollapsed(key);
              return (
                <tbody key={key}>
                  {/* The whole row toggles; the button's click bubbles up to it and keeps it keyboard-reachable. */}
                  <tr
                    className="bracket-list-group"
                    onClick={(e) => {
                      if (!(e.target as Element).closest("a")) toggle(key);
                    }}
                  >
                    <th scope="colgroup" colSpan={renderActions ? 7 : 6}>
                      <span className="bracket-list-group-head">
                        <button type="button" className="bracket-list-group-toggle" aria-expanded={!collapsed}>
                          <Icon name="chevronDown" size={18} className="bracket-list-group-caret" />
                          {group.stage?.name ?? "Stage'i belirlenmemiş"}
                          <span className="badge">{group.matches.length} maç</span>
                        </button>
                        {group.stage?.published && <Link to={`/mappool?stage=${group.stage.id}`}>Mappool</Link>}
                      </span>
                    </th>
                  </tr>
                  {!collapsed &&
                    group.matches.map((m) => {
                      const [s1, s2] = sources.get(m.id) ?? [null, null];
                      const winner = winningSlot(m);
                      const mine =
                        viewerId !== undefined && (m.referee?.id === viewerId || m.streamer?.id === viewerId);
                      const playing = myTeamId != null && (m.team1_id === myTeamId || m.team2_id === myTeamId);
                      const classes = [
                        m.id === selectedId && "is-selected",
                        (mine || playing) && "is-mine",
                        onSelect && "is-selectable",
                      ]
                        .filter(Boolean)
                        .join(" ");
                      return (
                        <tr
                          key={m.id}
                          className={classes}
                          // A click anywhere in the row selects the match, except on its own controls; the
                          // code stays a button so the keyboard can reach it.
                          onClick={
                            onSelect &&
                            ((e) => {
                              if ((e.target as Element).closest(ROW_CONTROLS)) return;
                              if (window.getSelection()?.toString()) return;
                              onSelect(m.id);
                            })
                          }
                        >
                          <th scope="row">
                            {onSelect ? (
                              <button
                                type="button"
                                className={`bracket-list-code is-${m.side}`}
                                onClick={() => onSelect(m.id)}
                              >
                                {matchCode(m)}
                              </button>
                            ) : (
                              <span className={`bracket-list-code is-${m.side}`}>{matchCode(m)}</span>
                            )}
                          </th>
                          <td className="bracket-list-time" data-label="Zaman">
                            {!m.starts_at ? (
                              <span className="muted">—</span>
                            ) : onShowInCalendar ? (
                              <MatchTimeLink
                                matchId={m.id}
                                startsAt={m.starts_at}
                                countdown={countdown}
                                onShowInCalendar={onShowInCalendar}
                              />
                            ) : (
                              <MatchTime startsAt={m.starts_at} countdown={countdown} />
                            )}
                          </td>
                          <td className="bracket-list-pairing-cell">
                            <div className="bracket-list-pairing">
                              <TeamName id={m.team1_id} source={s1} teams={teams} won={winner === 1} own={m.team1_id != null && m.team1_id === myTeamId} bannerEnd />
                              <span className="bracket-list-score">
                                <span className={winner === 1 ? "is-winner" : undefined}>{m.team1_score ?? "–"}</span>
                                <span className={winner === 2 ? "is-winner" : undefined}>{m.team2_score ?? "–"}</span>
                              </span>
                              <TeamName id={m.team2_id} source={s2} teams={teams} won={winner === 2} own={m.team2_id != null && m.team2_id === myTeamId} />
                            </div>
                          </td>
                          <td className="bracket-list-referee" data-label="Hakem">
                            {renderReferee ? (
                              renderReferee(m)
                            ) : m.referee ? (
                              <span className="bracket-list-person">
                                {m.referee.username}
                                <DiscordMention player={m.referee} fallback="" />
                              </span>
                            ) : (
                              <span className="muted">—</span>
                            )}
                          </td>
                          <td className="bracket-list-streamer" data-label="Yayıncı">
                            {renderStreamer ? (
                              renderStreamer(m)
                            ) : m.streamer ? (
                              m.streamer.username
                            ) : (
                              <span className="muted">—</span>
                            )}
                          </td>
                          <td className="bracket-list-status-cell">
                            <span className="bracket-list-status">
                              <MatchStatus match={m} now={now} />
                              <RoomLinks rooms={m.rooms} />
                            </span>
                          </td>
                          {renderActions && (
                            <td className="bracket-list-actions-cell">
                              <div className="bracket-list-actions">{renderActions(m)}</div>
                            </td>
                          )}
                        </tr>
                      );
                    })}
                </tbody>
              );
            })}
          </table>
        </div>
      )}
    </div>
  );
}

interface TeamNameProps {
  id: string | null;
  source: Source | null;
  teams: Map<string, Team>;
  won: boolean;
  /** The viewer's own team. */
  own: boolean;
  /** Puts the banner after the name, next to the score, for the left-hand team. */
  bannerEnd?: boolean;
}

function TeamName({ id, source, teams, won, own, bannerEnd }: TeamNameProps) {
  const team = id ? teams.get(id) : undefined;
  const classes = ["bracket-list-team", !team && "is-empty", won && "is-winner", own && "is-own", bannerEnd && "is-banner-end"]
    .filter(Boolean)
    .join(" ");
  return (
    <span className={classes}>
      {team ? (
        <TeamPopover team={team}>
          <TeamBanner team={team} />
          {team.name}
        </TeamPopover>
      ) : (
        <>
          <TeamBanner />
          {source ? sourceLabel(source) : "Belirlenmedi"}
        </>
      )}
    </span>
  );
}

export function MatchStatus({ match, now }: { match: BracketMatch; now: string }) {
  if (match.concluded_at) return <span className="badge badge-success">Tamamlandı</span>;
  if (!match.starts_at) return <span className="badge">Zaman yok</span>;
  if (match.starts_at <= now) return <span className="badge badge-warning">Başladı</span>;
  return <span className="badge">Planlandı</span>;
}
