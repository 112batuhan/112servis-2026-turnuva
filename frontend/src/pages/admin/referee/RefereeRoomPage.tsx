import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import type { RefereeViewer } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { useLive } from "../../../live/LiveContext";
import { normalizeName, type SlotTeam } from "../../../lib/bancho";
import { formatMatchTime } from "../../../lib/format";
import { ChatPanel } from "./ChatPanel";
import { LobbyPanel } from "./LobbyPanel";
import { poolEntries } from "../../../lib/matchPool";
import { PoolPanel } from "./PoolPanel";
import { TurnStatus } from "../../../components/match/PoolView";
import { roomLabel } from "../../../lib/liveRoom";
import { botBlocker, useRefereeRoom, type RefereeRoomState } from "./useRefereeRoom";
import { CockpitTabs, TABS, panelProps, useCockpit } from "../../../components/match/Cockpit";

/**
 * /admin/referee/:matchId — runs a match's osu! room. The site's bot makes and runs the room; the
 * referee drives it from here and talks in it through the bot.
 */
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

  const { tab, select, compact, pageRef, barRef, tabsRef } = useCockpit(room !== null);
  const [chatUnread, setChatUnread] = useState(0);

  const colors = useMemo(() => {
    const map = new Map<string, SlotTeam | null>();
    for (const r of roster) map.set(normalizeName(r.player.username), r.color);
    return map;
  }, [roster]);

  if (loadError && !room) {
    return (
      <div className="referee-page osu-team-colors">
        <ErrorAlert message={loadError} />
        {/* Reopens the tab the referee came from. */}
        <Link to="/admin">← Geri dön</Link>
      </div>
    );
  }
  if (!room) return <p className="status">Yükleniyor…</p>;

  const teamNames =
    room.kind === "bracket"
      ? [room.team1_id, room.team2_id]
          .map((id) => room.teams.find((t) => t.id === id)?.name ?? "Belirlenmedi")
          .join(" vs ")
      : `· ${room.teams.length} takım`;
  // With both teams known, the match column's score names them.
  const strip = referee.tally !== null && referee.teams.length === 2;

  return (
    <div className="referee-page osu-team-colors" ref={pageRef}>
      <header className="referee-header">
        <Link
          to={room.kind === "bracket" ? "/admin/bracket" : "/admin/qualifiers"}
          className="referee-back"
        >
          {room.kind === "bracket" ? "← Bracket" : "← Elemeler"}
        </Link>
        <h1 className="referee-title">
          {roomLabel(room)}{" "}
          <span className={strip ? "sr-only" : "referee-title-teams"}>{teamNames}</span>
        </h1>
        <span className="referee-meta">
          {room.starts_at && formatMatchTime(room.starts_at)}
          {room.referee ? ` · Hakem: ${room.referee.username}` : " · Hakem yok"}
          {room.streamer && ` · Yayıncı: ${room.streamer.username}`}
        </span>
        {room.concluded_at && <span className="badge badge-success">Bitti</span>}
        {!referee.canRun && (
          <span
            className="badge badge-warning"
            title="Odayı sadece maçın hakemi ya da bir host kurabilir"
          >
            Bu maçın hakemi değilsin
          </span>
        )}
        <PageViewers viewers={presence.get(matchId)} />
      </header>

      {/* On a phone this sticks under the top bar: the tabs, and an error where it can't scroll away. */}
      <div className="referee-bar" ref={barRef}>
        <CockpitTabs
          tabsRef={tabsRef}
          label="Hakem paneli"
          active={tab}
          onSelect={select}
          status={{
            pool: <PoolStatus referee={referee} />,
            room: <RoomStatus referee={referee} />,
            chat: <ChatStatus unread={chatUnread} ready={referee.botReady} />,
          }}
        />
        <ErrorAlert message={error} />
      </div>

      {/* Pool | match | chat: everything a map needs in one screen, each column scrolling on its own. */}
      <div className="referee-cockpit" data-tab={tab}>
        {TABS.map(({ id }) => (
          <div key={id} className={`referee-cockpit-${id}`} {...(compact && panelProps(id))}>
            {id === "pool" && <PoolPanel referee={referee} />}
            {id === "room" && <LobbyPanel referee={referee} />}
            {id === "chat" && (
              <ChatPanel
                rooms={roomChats}
                colors={colors}
                say={{
                  bot: referee.botName,
                  blocker: botBlocker(referee),
                  auto: referee.autoLock !== null,
                  send: referee.actions.say,
                }}
                visible={!compact || tab === "chat"}
                onUnread={setChatUnread}
                log={referee.autorefLog}
              />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

/** The draft's phase: the next ban or pick by the stage's order, its team in their colour. */
function PoolStatus({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, turn, race } = referee;
  if (!room) return null;
  const maps = poolEntries(room).length;
  if (
    room.kind !== "bracket" ||
    teams.length !== 2 ||
    maps === 0 ||
    room.concluded_at ||
    race?.kind === "won"
  ) {
    return <span className="num">{maps} map</span>;
  }
  if (race?.kind === "tiebreaker") return <span>Tiebreaker</span>;
  if (!turn) return <span className="num">{maps} map</span>;
  return <TurnStatus turn={turn} />;
}

/** The running score in the sides' colours (a qualifier: who's in the room), lit while a map plays. */
function RoomStatus({ referee }: { referee: RefereeRoomState }) {
  const { room, tally, teams, lobby } = referee;
  if (!room) return null;
  const live = lobby.playing && !room.concluded_at;
  let text;
  if (room.kind === "bracket" && tally && teams.length === 2) {
    text = (
      <span className="referee-tab-score num">
        <span className="is-team-blue">{tally.team1}</span>
        <span aria-hidden="true">–</span>
        <span className="sr-only"> - </span>
        <span className="is-team-red">{tally.team2}</span>
      </span>
    );
  } else if (room.rooms.length === 0) {
    text = <span>Oda yok</span>;
  } else {
    text = (
      <span>
        <span className="num">{lobby.slots.length}</span> oyuncu
      </span>
    );
  }
  return (
    <>
      {live && <span className="referee-tab-live" title="Map oynanıyor" />}
      {live && <span className="sr-only">Map oynanıyor, </span>}
      {text}
    </>
  );
}

/** New lines waiting, or else whether the room's bot takes what the referee writes. */
function ChatStatus({ unread, ready }: { unread: number; ready: boolean }) {
  if (unread > 0) return <span className="referee-tab-unread num">{unread} yeni</span>;
  if (ready) return <span className="referee-tab-online">Bot ile</span>;
  return <span>Okunuyor</span>;
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
        {others === 0 ? "Sayfada sadece sen" : `Sayfada ${viewers.length} kişi`}
      </span>
      <ul>
        {viewers.map((v) => (
          <li key={v.id} className={v.id === user.id ? "is-self" : undefined} title={v.username}>
            <img src={v.avatar_url} alt="" />
            <span className="referee-viewers-name">
              {v.username}
              {v.id === user.id && <span className="referee-viewers-self"> (sen)</span>}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
