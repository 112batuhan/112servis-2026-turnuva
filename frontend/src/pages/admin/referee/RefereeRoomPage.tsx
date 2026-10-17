import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";
import { Link, useParams } from "react-router-dom";
import type { RefereeViewer } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { useIrc } from "../../../irc/IrcContext";
import { useLive } from "../../../live/LiveContext";
import { normalizeName, type SlotTeam } from "../../../lib/bancho";
import { formatMatchTime } from "../../../lib/format";
import { ChatPanel } from "./ChatPanel";
import { IrcLogin } from "./IrcLogin";
import { LobbyPanel } from "./LobbyPanel";
import { BAN_LIMIT, poolEntries } from "./pool";
import { PoolPanel } from "./PoolPanel";
import { roomLabel, useRefereeRoom, type RefereeRoomState } from "./useRefereeRoom";
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

  const { tab, select, compact, pageRef, barRef, tabsRef } = useCockpit(room !== null);
  const [chatUnread, setChatUnread] = useState(0);

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
      : `· ${room.teams.length} takım`;
  // With both teams known, the match column's score names them.
  const strip = referee.tally !== null && referee.teams.length === 2;

  return (
    <div className="referee-page" ref={pageRef}>
      <header className="referee-header">
        <Link to={room.kind === "bracket" ? "/admin/bracket" : "/admin/qualifiers"} className="referee-back">
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
        {room.concluded_at && <span className="badge badge-success">Bitti</span>}        {!referee.canRun && (
          <span className="badge badge-warning" title="Odayı sadece maçın hakemi ya da bir host kurabilir">
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
            chat: <ChatStatus unread={chatUnread} />,
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
                top={<IrcLogin />}
                visible={!compact || tab === "chat"}
                onUnread={setChatUnread}
              />
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

export type CockpitTab = "pool" | "room" | "chat";

export const TABS: { id: CockpitTab; label: string }[] = [
  { id: "pool", label: "Havuz" },
  { id: "room", label: "Maç" },
  { id: "chat", label: "Sohbet" },
];

/**
 * The cockpit's layout: on a phone one panel at a time under a tab bar; wider, every panel shows
 * and the bar is hidden. The CSS decides which, so this reads it back from the bar. Shared by the
 * referee page and the public match page.
 */
export function useCockpit(loaded: boolean) {
  const [tab, setTab] = useState<CockpitTab>("room");
  const [compact, setCompact] = useState(false);
  const pageRef = useRef<HTMLDivElement>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const tabsRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const page = pageRef.current;
    const bar = barRef.current;
    const tabs = tabsRef.current;
    if (!page || !bar || !tabs) return;
    const update = () => {
      setCompact(tabs.offsetHeight > 0);
      page.style.setProperty("--referee-bar-height", `${bar.offsetHeight}px`);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(bar);
    return () => observer.disconnect();
  }, [loaded]);

  // Picking a tab brings the bar up under the top bar, so the panel gets the whole screen.
  const select = useCallback((next: CockpitTab) => {
    setTab(next);
    const bar = barRef.current;
    if (!bar) return;
    const stuckAt = parseFloat(getComputedStyle(bar).top) || 0;
    const offset = bar.getBoundingClientRect().top - stuckAt;
    if (offset > 1) {
      const smooth = !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      window.scrollBy({ top: offset, behavior: smooth ? "smooth" : "auto" });
    }
  }, []);

  return { tab, select, compact, pageRef, barRef, tabsRef };
}

export const panelProps = (id: CockpitTab) => ({
  role: "tabpanel",
  id: `referee-panel-${id}`,
  "aria-labelledby": `referee-tab-${id}`,
});

interface CockpitTabsProps {
  tabsRef: RefObject<HTMLDivElement>;
  /** The tab list's accessible name. */
  label: string;
  active: CockpitTab;
  onSelect: (tab: CockpitTab) => void;
  /** Where each panel stands, in one short line. */
  status: Record<CockpitTab, ReactNode>;
}

/**
 * The phone's cockpit tabs. Each one carries where its panel stands (the draft, the score, the
 * chat), so nobody has to switch to look.
 */
export function CockpitTabs({ tabsRef, label: listLabel, active, onSelect, status }: CockpitTabsProps) {
  function onKeyDown(e: KeyboardEvent) {
    const at = TABS.findIndex((t) => t.id === active);
    const to =
      e.key === "ArrowRight" ? (at + 1) % TABS.length
      : e.key === "ArrowLeft" ? (at + TABS.length - 1) % TABS.length
      : e.key === "Home" ? 0
      : e.key === "End" ? TABS.length - 1
      : null;
    if (to === null) return;
    e.preventDefault();
    onSelect(TABS[to].id);
    document.getElementById(`referee-tab-${TABS[to].id}`)?.focus();
  }

  return (
    <div className="referee-tabs" role="tablist" aria-label={listLabel} ref={tabsRef} onKeyDown={onKeyDown}>
      {TABS.map(({ id, label }) => (
        <button
          key={id}
          type="button"
          role="tab"
          id={`referee-tab-${id}`}
          aria-selected={id === active}
          aria-controls={`referee-panel-${id}`}
          tabIndex={id === active ? 0 : -1}
          className="referee-tab"
          onClick={() => onSelect(id)}
        >
          <span className="referee-tab-label">{label}</span>
          <span className="referee-tab-status">{status[id]}</span>
        </button>
      ))}
    </div>
  );
}

/** The draft's phase: bans so far, then whose pick is next in their team's colour. */
function PoolStatus({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, nextPick, race } = referee;
  if (!room) return null;
  const maps = poolEntries(room).length;
  if (room.kind !== "bracket" || teams.length !== 2 || maps === 0 || room.concluded_at || race?.kind === "won") {
    return <span className="num">{maps} map</span>;
  }
  if (race?.kind === "tiebreaker") return <span>Tiebreaker</span>;
  if (room.bans.length < BAN_LIMIT) {
    return (
      <span>
        Ban <span className="num">{room.bans.length}/{BAN_LIMIT}</span>
      </span>
    );
  }
  if (!nextPick) return <span>Pick sırası</span>;
  return <span className={`referee-tab-team is-team-${nextPick.color}`}>{nextPick.name}</span>;
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

/** New lines waiting, or else whether Bancho is connected. */
function ChatStatus({ unread }: { unread: number }) {
  const { status } = useIrc();
  if (unread > 0) return <span className="referee-tab-unread num">{unread} yeni</span>;
  if (status === "online") return <span className="referee-tab-online">Bağlı</span>;
  return <span>{status === "connecting" ? "Bağlanıyor…" : "Bağlı değil"}</span>;
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
