import { useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { BracketMatch, RefereeViewer, Team } from "../../api/types";
import {
  compareByTime,
  currentStageId,
  groupByStage,
  hasMatchPage,
  incomingSources,
  isUndecided,
  matchCode,
  potentialRows,
  rowMatchId,
  UNDECIDED_TIME,
  sourceLabel,
  isLive,
  scoreHalves,
  winningSlot,
  type Source,
} from "../../lib/bracket";
import { StageTabs } from "../StageTabs";
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

/** The stage tab of the matches that have no stage yet. */
const NO_STAGE = "none";

const stageKey = (m: BracketMatch) => m.stage?.id ?? NO_STAGE;

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
 * One stage's matches at a time, in time order, unscheduled ones last. A match still waiting on its week's
 * results is followed by its potential matches, each with its own time and staff. It opens on the stage being
 * played (`currentStageId`; the last stage once every one is over) and follows it as the bracket moves on, until
 * another is picked on the stage track (not remembered). The search narrows it to its hits.
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
  // Every stage that has a match, in play order, for the track; with only one there's nothing to pick.
  const stages = useMemo(
    () =>
      groupByStage(matches).map((g) => ({
        id: g.stage?.id ?? NO_STAGE,
        name: g.stage?.name ?? "Stage'siz",
        published: true,
        is_qualifier: false,
        best_of: g.stage?.best_of ?? null,
      })),
    [matches],
  );
  const currentId = useMemo(() => currentStageId(matches), [matches]);
  const live = useMemo(() => {
    const counts = new Map<string, number>();
    for (const m of matches.filter((m) => isLive(m))) counts.set(stageKey(m), (counts.get(stageKey(m)) ?? 0) + 1);
    return counts;
  }, [matches]);
  // null follows the stage being played; a picked stage that has lost its last match does too.
  const [pickedStage, setPickedStage] = useState<string | null>(null);
  const stage =
    [pickedStage, currentId].find((id) => stages.some((s) => s.id === id)) ?? stages[stages.length - 1]?.id;
  const shown = useMemo(
    () => matches.filter((m) => stageKey(m) === stage && (!hits || hits.has(m.id))).sort(compareByTime),
    [matches, stage, hits],
  );
  // From every match, so a slot fed from the other side still names its source.
  const sources = useMemo(() => incomingSources(matches), [matches]);
  const now = new Date().toISOString();

  const wrapRef = useRef<HTMLDivElement>(null);
  const focused = useRef<{ id: string } | null>(null);
  // The row to scroll to once it's rendered, after its stage has been opened.
  const scrollTo = useRef<string | null>(null);
  useEffect(() => {
    const m = focusOn && matches.find((m) => m.id === focusOn.id);
    if (!m || focused.current === focusOn) return;
    focused.current = focusOn;
    if (hits && !hits.has(m.id)) return;
    if (stageKey(m) !== stage) setPickedStage(stageKey(m));
    scrollTo.current = m.id;
  }, [focusOn, matches, hits, stage]);
  useEffect(() => {
    const id = scrollTo.current;
    const row = id && wrapRef.current?.querySelector(`tr[data-match-id="${CSS.escape(id)}"]`);
    if (!row) return;
    scrollTo.current = null;
    row.scrollIntoView({ block: "center" });
  });

  return (
    <div className="bracket-list-wrap" ref={wrapRef}>
      {stages.length > 1 && (
        <div className="bracket-list-toolbar">
          <StageTabs
            stages={stages}
            selectedId={stage ?? null}
            onSelect={setPickedStage}
            currentId={currentId}
            live={live}
          />
        </div>
      )}
      {shown.length === 0 ? (
        <p className="status">{stages.length > 1 ? "Bu stage'de aramayla eşleşen maç yok." : "Aramayla eşleşen maç yok."}</p>
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
            <tbody>
              {shown.flatMap((m) => [
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
