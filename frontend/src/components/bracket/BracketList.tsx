import { useMemo, type ReactNode } from "react";
import { Link } from "react-router-dom";
import type { BracketMatch, Team } from "../../api/types";
import { groupByStage, incomingSources, matchCode, sourceLabel, winningSlot, type Source } from "../../lib/bracket";
import { formatShortMatchTime } from "../../lib/format";
import { RoomLinks } from "../RoomLinks";
import { DiscordMention } from "../DiscordMention";
import "./BracketList.css";

interface Props<M extends BracketMatch> {
  matches: M[];
  teams: Map<string, Team>;
  myTeamId?: string | null;
  viewerId?: string;
  selectedId?: string | null;
  onSelect?: (id: string) => void;
  /** Controls at the end of each row. */
  renderActions?: (match: M) => ReactNode;
}

/** Every match grouped by stage, in stage order; each stage in time order, unscheduled ones last. */
export function BracketList<M extends BracketMatch>({
  matches,
  teams,
  myTeamId,
  viewerId,
  selectedId,
  onSelect,
  renderActions,
}: Props<M>) {
  const groups = useMemo(() => groupByStage(matches), [matches]);
  const sources = useMemo(() => incomingSources(matches), [matches]);
  const now = new Date().toISOString();

  return (
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
        {groups.map((group) => (
          <tbody key={group.stage?.id ?? "none"}>
            <tr className="bracket-list-group">
              <th scope="colgroup" colSpan={renderActions ? 7 : 6}>
                <span className="bracket-list-group-head">
                  {group.stage?.name ?? "Stage'i belirlenmemiş"}
                  <span className="badge">{group.matches.length} maç</span>
                  {group.stage?.published && <Link to={`/mappool?stage=${group.stage.id}`}>Mappool</Link>}
                </span>
              </th>
            </tr>
            {group.matches.map((m) => {
              const [s1, s2] = sources.get(m.id) ?? [null, null];
              const winner = winningSlot(m);
              const mine = viewerId !== undefined && (m.referee?.id === viewerId || m.streamer?.id === viewerId);
              const playing = myTeamId != null && (m.team1_id === myTeamId || m.team2_id === myTeamId);
              const classes = [m.id === selectedId && "is-selected", (mine || playing) && "is-mine"]
                .filter(Boolean)
                .join(" ");
              return (
                <tr key={m.id} className={classes}>
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
                  <td className="bracket-list-time">
                    {m.starts_at ? formatShortMatchTime(m.starts_at) : <span className="muted">—</span>}
                  </td>
                  <td>
                    <div className="bracket-list-pairing">
                      <TeamName id={m.team1_id} source={s1} teams={teams} won={winner === 1} />
                      <span className="bracket-list-score">
                        {m.team1_score ?? "–"} : {m.team2_score ?? "–"}
                      </span>
                      <TeamName id={m.team2_id} source={s2} teams={teams} won={winner === 2} />
                    </div>
                  </td>
                  <td>
                    {m.referee ? (
                      <span className="bracket-list-person">
                        {m.referee.username}
                        <DiscordMention player={m.referee} fallback="" />
                      </span>
                    ) : (
                      <span className="muted">—</span>
                    )}
                  </td>
                  <td>{m.streamer ? m.streamer.username : <span className="muted">—</span>}</td>
                  <td>
                    <span className="bracket-list-status">
                      <MatchStatus match={m} now={now} />
                      <RoomLinks rooms={m.rooms} />
                    </span>
                  </td>
                  {renderActions && (
                    <td>
                      <div className="bracket-list-actions">{renderActions(m)}</div>
                    </td>
                  )}
                </tr>
              );
            })}
          </tbody>
        ))}
      </table>
    </div>
  );
}

interface TeamNameProps {
  id: string | null;
  source: Source | null;
  teams: Map<string, Team>;
  won: boolean;
}

function TeamName({ id, source, teams, won }: TeamNameProps) {
  const team = id ? teams.get(id) : undefined;
  if (!team) {
    return <span className="bracket-list-team is-empty">{source ? sourceLabel(source) : "Belirlenmedi"}</span>;
  }
  return <span className={won ? "bracket-list-team is-winner" : "bracket-list-team"}>{team.name}</span>;
}

export function MatchStatus({ match, now }: { match: BracketMatch; now: string }) {
  if (match.concluded_at) return <span className="badge badge-success">Tamamlandı</span>;
  if (!match.starts_at) return <span className="badge">Zaman yok</span>;
  if (match.starts_at <= now) return <span className="badge badge-warning">Başladı</span>;
  return <span className="badge">Planlandı</span>;
}
