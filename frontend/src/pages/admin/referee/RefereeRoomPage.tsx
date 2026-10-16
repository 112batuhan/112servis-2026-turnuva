import { useEffect, useMemo } from "react";
import { Link, useParams } from "react-router-dom";
import type { RefereeViewer } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { useLive } from "../../../live/LiveContext";
import { normalizeName, type SlotTeam } from "../../../lib/bancho";
import { formatMatchTime } from "../../../lib/format";
import { ChatPanel } from "./ChatPanel";
import { IrcLogin } from "./IrcLogin";
import { LobbyPanel } from "./LobbyPanel";
import { PoolPanel } from "./PoolPanel";
import { RosterPanel } from "./RosterPanel";
import { roomLabel, useRefereeRoom } from "./useRefereeRoom";
import "./RefereeRoomPage.css";

/** /admin/referee/:matchId — runs a match's osu! room over Bancho IRC. */
export function RefereeRoomPage() {
  const { matchId = "" } = useParams();
  // A fresh page per match, so nothing from the last one (like a room still being
  // created) lands in the next.
  return <RefereeRoom key={matchId} matchId={matchId} />;
}

function RefereeRoom({ matchId }: { matchId: string }) {
  const referee = useRefereeRoom(matchId);
  const { room, roomChats, loadError, error, roster } = referee;
  // Tells the other staff this match's referee page is open here, for as long as it is.
  const { setViewing, presence } = useLive();
  useEffect(() => {
    setViewing(matchId);
    return () => setViewing(null);
  }, [matchId, setViewing]);

  const colors = useMemo(() => {
    const map = new Map<string, SlotTeam | null>();
    for (const r of roster) map.set(normalizeName(r.player.username), r.color);
    return map;
  }, [roster]);

  if (loadError && !room) {
    return (
      <div className="referee-page">
        <ErrorAlert message={loadError} />
        {/* Reopens the tab the referee came from. */}
        <Link to="/admin">← Geri dön</Link>
      </div>
    );
  }
  if (!room) return <p className="status">Yükleniyor…</p>;

  const teamNames =
    room.kind === "bracket"
      ? [room.team1_id, room.team2_id].map((id) => room.teams.find((t) => t.id === id)?.name ?? "Belirlenmedi").join(" vs ")
      : `${room.teams.length} takım`;

  return (
    <div className="referee-page">
      <header className="referee-header">
        <Link to={room.kind === "bracket" ? "/admin/bracket" : "/admin/qualifiers"} className="referee-back">
          {room.kind === "bracket" ? "← Bracket" : "← Elemeler"}
        </Link>
        <h1 className="referee-title">
          {roomLabel(room)} <span className="referee-title-teams">{teamNames}</span>
        </h1>
        <span className="referee-meta">
          {room.starts_at && formatMatchTime(room.starts_at)}
          {room.referee ? ` · Hakem: ${room.referee.username}` : " · Hakem yok"}
        </span>
        {room.concluded_at && <span className="badge badge-success">Bitti</span>}
        {!referee.canRun && (
          <span className="badge badge-warning" title="Odayı sadece maçın hakemi ya da bir host kurabilir">
            Bu maçın hakemi değilsin
          </span>
        )}
      </header>

      <PageViewers viewers={presence.get(matchId)} />
      <IrcLogin />
      <ErrorAlert message={error} />

      <div className="referee-grid">
        <div className="referee-column">
          <RosterPanel referee={referee} />
        </div>
        <div className="referee-column referee-main">
          <LobbyPanel referee={referee} />
          <ChatPanel rooms={roomChats} colors={colors} />
        </div>
        <div className="referee-column">
          <PoolPanel referee={referee} />
        </div>
      </div>
    </div>
  );
}

/**
 * Everyone with this referee page open right now, the viewer included, so two referees (or a host
 * looking in) know they aren't alone on the match.
 */
function PageViewers({ viewers }: { viewers: RefereeViewer[] | undefined }) {
  const user = useSignedInUser();
  if (!viewers || viewers.length === 0) return null;
  const others = viewers.filter((v) => v.id !== user.id).length;
  return (
    <section className="referee-viewers" aria-label="Bu sayfadakiler">
      <span className="referee-viewers-label">
        {others === 0 ? "Bu sayfada sadece sen varsın" : `Bu sayfada ${viewers.length} kişi`}
      </span>
      <ul>
        {viewers.map((v) => (
          <li key={v.id} className={v.id === user.id ? "is-self" : undefined}>
            <img src={v.avatar_url} alt="" />
            {v.username}
            {v.id === user.id && <span className="referee-viewers-self"> (sen)</span>}
          </li>
        ))}
      </ul>
    </section>
  );
}
