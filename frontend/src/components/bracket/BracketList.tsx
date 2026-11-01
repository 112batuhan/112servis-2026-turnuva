import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import type { BracketMatch, BracketSide, RefereeViewer, Team } from "../../api/types";
import {
  groupByStage,
  hasMatchPage,
  incomingSources,
  isUndecided,
  matchCode,
  potentialRows,
  rowMatchId,
  UNDECIDED_TIME,
  SIDE_LABELS,
  sourceLabel,
  isLive,
  scoreHalves,
  winningSlot,
  type Source,
} from "../../lib/bracket";
import { stageLabel } from "../../lib/bestOf";
import { readString, writeString } from "../../lib/storage";
import { useStoredSet } from "../../lib/useExpandedSet";
import { Icon } from "../Icon";
import { MatchPageLink } from "../MatchPageLink";
import { RoomLinks } from "../RoomLinks";
import { StatusChip } from "../StatusChip";
import { TeamPopover } from "../TeamPopover";
import { MatchTime } from "./MatchTime";
import { MatchTimeLink } from "./MatchTimeLink";
import { RefereeWatchers } from "./RefereeWatchers";
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
  /** Called with null when the selected match is clicked again. */
  onSelect?: (id: string | null) => void;
  /**
   * Scrolls this match's row to the middle, again each time a new object is passed, first opening its stage and
   * showing both sides if they hide it. A match the search leaves out stays out.
   */
  focusOn?: { id: string } | null;
  /** Replaces the referee's name and Discord, e.g. with controls to change who it is. */
  renderReferee?: (match: M) => ReactNode;
  /** Replaces the streamer's name, likewise. */
  renderStreamer?: (match: M) => ReactNode;
  /** Controls at the end of each row. */
  renderActions?: (match: M) => ReactNode;
  /**
   * Replaces the match page link in the status cell, in its column, e.g. with the referee page's.
   * Not called for potential matches, which have neither.
   */
  renderPageLink?: (match: M) => ReactNode;
  /** Makes each time a button that opens the calendar on that match. */
  onShowInCalendar?: (id: string) => void;
  /** Shows the time left until each match instead of its date. */
  countdown?: boolean;
  /** The ids of the matches the search finds, or null when not searching. */
  hits?: Set<string> | null;
  /** Who has each match's referee page open, by match id (admin only). */
  presence?: ReadonlyMap<string, RefereeViewer[]>;
}

/**
 * Every match grouped by stage, in stage order; each stage in time order, unscheduled ones last. A match still
 * waiting on its week's results is followed by its potential matches, each with its own time and staff.
 * Stages can be collapsed and the list narrowed to one side (both remembered per browser) or to the search's hits.
 */
export function BracketList<M extends BracketMatch>({
  matches,
  teams,
  myTeamId,
  viewerId,
  selectedId,
  onSelect,
  focusOn,
  renderReferee,
  renderStreamer,
  renderActions,
  renderPageLink,
  onShowInCalendar,
  countdown,
  hits,
  presence,
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

  const wrapRef = useRef<HTMLDivElement>(null);
  const focused = useRef<{ id: string } | null>(null);
  // The row to scroll to once it's rendered, after the stage or side hiding it has been opened.
  const scrollTo = useRef<string | null>(null);
  useEffect(() => {
    const m = focusOn && matches.find((m) => m.id === focusOn.id);
    if (!m || focused.current === focusOn) return;
    focused.current = focusOn;
    if (hits && !hits.has(m.id)) return;
    // Shown for now without changing the side remembered for next time.
    if (side !== "all" && m.side !== side) setSide("all");
    const key = m.stage?.id ?? "none";
    if (isCollapsed(key)) toggle(key);
    scrollTo.current = m.id;
  }, [focusOn, matches, hits, side, isCollapsed, toggle]);
  useEffect(() => {
    const id = scrollTo.current;
    const row = id && wrapRef.current?.querySelector(`tr[data-match-id="${CSS.escape(id)}"]`);
    if (!row) return;
    scrollTo.current = null;
    row.scrollIntoView({ block: "center" });
  });

  return (
    <div className="bracket-list-wrap" ref={wrapRef}>
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
                <th scope="col" colSpan={2}>
                  Durum
                </th>
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
                    <th scope="colgroup" colSpan={renderActions ? 8 : 7}>
                      <span className="bracket-list-group-head">
                        <button type="button" className="bracket-list-group-toggle" aria-expanded={!collapsed}>
                          <Icon name="chevronDown" size={18} className="bracket-list-group-caret" />
                          {group.stage ? stageLabel(group.stage) : "Stage'i belirlenmemiş"}
                          <span className="badge">{group.matches.length} maç</span>
                        </button>
                        {group.stage?.published && <Link to={`/mappool?stage=${group.stage.id}`}>Mappool</Link>}
                      </span>
                    </th>
                  </tr>
                  {!collapsed &&
                    group.matches.flatMap((m) => [
                      m,
                      ...potentialRows(m).filter((p) => !hits || hits.has(p.id)),
                    ]).map((m) => {
                      const potential = m.potential_of !== undefined;
                      const undecided = isUndecided(m);
                      const [s1, s2] = potential ? [null, null] : (sources.get(m.id) ?? [null, null]);
                      const winner = winningSlot(m);
                      const halves = scoreHalves(m);
                      const mine =
                        viewerId !== undefined && (m.referee?.id === viewerId || m.streamer?.id === viewerId);
                      const playing = myTeamId != null && (m.team1_id === myTeamId || m.team2_id === myTeamId);
                      const classes = [
                        m.id === selectedId && "is-selected",
                        (mine || playing) && "is-mine",
                        onSelect && "is-selectable",
                        potential && "is-potential",
                        undecided && "has-potentials",
                      ]
                        .filter(Boolean)
                        .join(" ");
                      // A potential match selects its bracket match, which the grid and the panel know; clicking
                      // the selected match again deselects it.
                      const id = rowMatchId(m);
                      const select = () => onSelect?.(id === selectedId ? null : id);
                      return (
                        <tr
                          key={m.id}
                          className={classes}
                          data-match-id={m.id}
                          // A click anywhere in the row selects the match, except on its own controls; the
                          // code stays a button so the keyboard can reach it.
                          onClick={
                            onSelect &&
                            ((e) => {
                              if ((e.target as Element).closest(ROW_CONTROLS)) return;
                              if (window.getSelection()?.toString()) return;
                              select();
                            })
                          }
                        >
                          <th scope="row">
                            {potential ? (
                              <span className="bracket-list-branch">
                                <span className={`bracket-list-branch-code is-${m.side}`}>{matchCode(m)}</span>
                                olası
                              </span>
                            ) : onSelect ? (
                              <button
                                type="button"
                                className={`bracket-list-code is-${m.side}`}
                                aria-pressed={m.id === selectedId}
                                onClick={select}
                              >
                                {matchCode(m)}
                              </button>
                            ) : (
                              <span className={`bracket-list-code is-${m.side}`}>{matchCode(m)}</span>
                            )}
                          </th>
                          <td className="bracket-list-time" data-label="Zaman">
                            {undecided ? (
                              <span className="bracket-list-undecided">{UNDECIDED_TIME}</span>
                            ) : !m.starts_at ? (
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
                              <span className={isLive(m) ? "bracket-list-score is-live" : "bracket-list-score"}>
                                <span className={winner === 1 ? "is-winner" : undefined}>{halves?.[0] ?? "–"}</span>
                                <span className={winner === 2 ? "is-winner" : undefined}>{halves?.[1] ?? "–"}</span>
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
                                <DiscordMention player={m.referee} fallback="" compact />
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
                          {/* The state and the links are cells of their own, so the links line up in their
                              column, and a phone can put the state on the block's top line by the code. */}
                          <td className="bracket-list-state-cell">
                            {potential ? (
                              <StatusChip state="potential" title="İki takım da gelirse oynanır">
                                Olası
                              </StatusChip>
                            ) : undecided ? (
                              <StatusChip state="potential">{m.potentials.length} olası eşleşme</StatusChip>
                            ) : (
                              <MatchStatus match={m} now={now} />
                            )}
                          </td>
                          <td className="bracket-list-status-cell">
                            <span className="bracket-list-status">
                              {renderPageLink
                                ? !potential && renderPageLink(m)
                                : hasMatchPage(m) && <MatchPageLink matchId={m.id} live={isLive(m)} variant="text" />}
                              <RoomLinks rooms={m.rooms} />
                              {!potential && <RefereeWatchers viewers={presence?.get(m.id)} />}
                            </span>
                          </td>
                          {renderActions && (
                            <td className="bracket-list-actions-cell">
                              {/* A potential match has no room or referee page yet. */}
                              {!potential && <div className="bracket-list-actions">{renderActions(m)}</div>}
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
  if (match.concluded_at && match.forfeit_slot !== null) {
    return (
      <StatusChip state="done" title="Bir takım çekildi ya da maç tamamlanamadı">
        Hükmen
      </StatusChip>
    );
  }
  if (match.concluded_at) return <StatusChip state="done">Tamamlandı</StatusChip>;
  if (isLive(match, new Date(now).getTime())) return <StatusChip state="live">Canlı</StatusChip>;
  if (!match.starts_at) return <StatusChip state="none">Zaman yok</StatusChip>;
  if (match.starts_at <= now) {
    return (
      <StatusChip state="due" title="Maçın saati geldi, odası henüz açılmadı">
        Başladı
      </StatusChip>
    );
  }
  return <StatusChip state="waiting">Planlandı</StatusChip>;
}
