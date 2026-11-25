import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
} from "react";
import { Link, useParams } from "react-router-dom";
import { playerAction } from "../../../../api/matches";
import type { PlayerAction } from "../../../../api/types";
import { useSignedInUser } from "../../../../auth/AuthContext";
import { ErrorAlert } from "../../../../components/ErrorAlert";
import { Icon } from "../../../../components/Icon";
import { LiveMark } from "../../../../components/LiveMark";
import { CockpitTabs, panelProps, useCockpit } from "../../../../components/match/Cockpit";
import { ScoreSheet } from "../../../../components/match/ScoreSheet";
import { normalizeName, type SlotTeam } from "../../../../lib/bancho";
import { stageLabel } from "../../../../lib/bestOf";
import { matchPagePath } from "../../../../lib/bracket";
import type { StepKind } from "../../../../lib/draftOrder";
import { formatMatchTime } from "../../../../lib/format";
import { roomLabel } from "../../../../lib/liveRoom";
import type { PoolEntry } from "../../../../lib/matchPool";
import { clock, shownEnd } from "../../../../lib/playerRoom";
import { useAction } from "../../../../lib/useAction";
import { useMatchPage, type MatchPageState } from "../../../../lib/useMatchPage";
import { MatchChat } from "../MatchChat";
import { RoomActions } from "./RoomActions";
import { RoomPool } from "./RoomPool";
import { TurnBar } from "./TurnBar";
import "../../../../components/match/Cockpit.css";
import "../../../../components/match/ScoreSheet.css";
import "../MatchPage.css";
import "./PlayerRoomPage.css";

/** The viewer as a player of the match: their team, and its side in a bracket match. */
export interface RoomMe {
  userId: string;
  osuId: number;
  teamId: string;
  teamName: string;
  color: SlotTeam | null;
}

/**
 * /match/:matchId/players — the room page of a match's own players, beside the osu! client: who plays
 * (and, while the room is open, who's in it), the pool
 * they ban and pick from (held for three seconds), the team's warmup, whose turn it is with its
 * clock, the score, abort / rapor / durdur, and the room's chat as it happens (read only). Bans,
 * picks and warmups are only taken here; the room's bot runs the rest.
 */
export function PlayerRoomPage() {
  const { matchId = "" } = useParams();
  return <PlayerRoom key={matchId} matchId={matchId} />;
}

function PlayerRoom({ matchId }: { matchId: string }) {
  const user = useSignedInUser();
  // Players aren't spectators: the page follows the match without being counted among them.
  const page = useMatchPage(matchId, { spectate: false });
  const { match, loadError, roomChats, roster, connected, teams } = page;
  const { tab, select, compact, pageRef, barRef, tabsRef } = useCockpit(match !== null, "pool");
  const [chatUnread, setChatUnread] = useState(0);
  const [pending, setPending] = useState<string | null>(null);
  const { busy, error, setError, run } = useAction();
  const now = useNow(match !== null && !match.concluded_at);
  const gridRef = useRoomHeight(match !== null);

  const colors = useMemo(() => {
    const map = new Map<string, SlotTeam | null>();
    for (const r of roster) map.set(normalizeName(r.player.username), r.color);
    return map;
  }, [roster]);

  const me = useMemo<RoomMe | null>(() => {
    const team = match?.teams.find((t) => t.players.some((p) => p.id === user.id));
    const player = team?.players.find((p) => p.id === user.id);
    if (!team || !player) return null;
    return {
      userId: user.id,
      osuId: player.osu_id,
      teamId: team.id,
      teamName: team.name,
      color: teams.find((t) => t.id === team.id)?.color ?? null,
    };
  }, [match, user.id, teams]);

  const act = useCallback(
    (action: PlayerAction) => run(() => playerAction(matchId, action)),
    [run, matchId],
  );
  const choose = useCallback(
    async (entry: PoolEntry) => {
      const id = entry.map.id;
      setPending(id);
      const ok = await act({ type: "draft", map_id: id });
      // Taken, it's marked by the live update (below), or let go after a while if that's slow; a
      // refusal is in the error.
      if (!ok) setPending(null);
      else window.setTimeout(() => setPending((p) => (p === id ? null : p)), 8000);
    },
    [act],
  );

  // A hold began: the bot is told, so a turn that runs out under it waits for the hold to finish.
  // Nothing to show for it (a refused one is refused again when the hold ends), so it doesn't go
  // through `run`, which would mark the page busy and drop the hold's tile.
  const hold = useCallback(
    (entry: PoolEntry) => {
      playerAction(matchId, { type: "hold", map_id: entry.map.id }).catch(() => {});
    },
    [matchId],
  );

  // The ban or pick lands through the live socket; it's no longer pending once it's on the match.
  useEffect(() => {
    if (!pending || !match) return;
    if (
      match.bans.some((b) => b.map_id === pending) ||
      match.picks.some((p) => p.map_id === pending)
    )
      setPending(null);
  }, [match, pending]);

  if (!match) {
    if (loadError) {
      return (
        <div className="page referee-page match-page player-room osu-team-colors">
          <ErrorAlert message={loadError} />
          <Link to="/team">← Takımım</Link>
        </div>
      );
    }
    return <p className="status">Yükleniyor…</p>;
  }

  if (!me) {
    return (
      <div className="page page-narrow player-room-outsider">
        <div className="page-header">
          <h1>Maç odası</h1>
        </div>
        <p className="status">
          Bu sayfa maçın oyuncuları için: warmup, ban ve pick'ler buradan seçilir. Maçı seyirci
          sayfasından canlı izleyebilirsin.
        </p>
        <Link to={matchPagePath(match.id)} className="btn btn-primary">
          Maç sayfasına git
        </Link>
      </div>
    );
  }

  const autoref = page.liveRoom?.autoref ?? null;
  const myTurn: StepKind | null =
    autoref?.enabled && autoref.phase === "draft" && autoref.team_id === me.teamId
      ? (autoref.step ?? null)
      : null;
  const bracket = match.kind === "bracket";
  const teamName = (id: string | null) =>
    match.teams.find((t) => t.id === id)?.name ?? "Belirlenmedi";

  return (
    <div
      className="page referee-page match-page player-room osu-team-colors"
      ref={pageRef}
      style={
        me.color ? ({ "--room-own": `var(--color-team-${me.color})` } as CSSProperties) : undefined
      }
    >
      <header className="referee-header match-header">
        <div className="match-header-lead">
          <Link to={matchPagePath(match.id)} className="referee-back player-room-back">
            <Icon name="arrowLeft" size={14} />
            Maç sayfası
          </Link>
          <h1 className="referee-title">
            <span className={match.side ? `match-code is-${match.side}` : undefined}>
              {roomLabel(match)}
            </span>{" "}
            {bracket ? (
              <span className="referee-title-teams player-room-versus">
                <SideName
                  name={teamName(match.team1_id)}
                  side="blue"
                  own={me.teamId === match.team1_id}
                />{" "}
                vs{" "}
                <SideName
                  name={teamName(match.team2_id)}
                  side="red"
                  own={me.teamId === match.team2_id}
                />
              </span>
            ) : (
              <span className="referee-title-teams">· {match.teams.length} takım</span>
            )}
          </h1>
        </div>
        <p className="referee-meta match-meta">
          {match.starts_at && (
            <time className="match-meta-time" dateTime={match.starts_at}>
              {formatMatchTime(match.starts_at)}
            </time>
          )}
          {match.stage && (
            <span className="match-meta-value">
              {bracket ? stageLabel(match.stage) : match.stage.name}
            </span>
          )}
          <span>
            Hakem <span className="match-meta-value">{match.referee?.username ?? "henüz yok"}</span>
          </span>
        </p>
        <RoomRoster page={page} />
        {page.live ? (
          <div className="match-onair player-room-onair">
            <LiveMark large />
          </div>
        ) : (
          match.concluded_at && (
            <span className="match-done">{match.forfeit_slot ? "Hükmen bitti" : "Bitti"}</span>
          )
        )}
      </header>

      <div className="referee-bar" ref={barRef}>
        <CockpitTabs
          tabsRef={tabsRef}
          label="Maç odası"
          active={tab}
          onSelect={select}
          status={{
            pool: <PoolTabStatus page={page} me={me} now={now} />,
            room: <ScoreTabStatus page={page} />,
            chat:
              chatUnread > 0 ? (
                <span className="referee-tab-unread num">{chatUnread} yeni</span>
              ) : (
                <span>{connected || !page.live ? "Oda sohbeti" : "Bağlanıyor…"}</span>
              ),
          }}
        />
        <ErrorAlert message={error} onDismiss={() => setError(null)} />
        <ErrorAlert message={loadError} />
      </div>

      <div className="player-room-grid" data-tab={tab} ref={gridRef}>
        <div className="player-room-turn">
          <TurnBar
            page={page}
            me={me}
            now={now}
            busy={busy}
            onWarmup={(beatmap_id) => act({ type: "warmup", beatmap_id })}
            onSkipWarmup={() => act({ type: "skip_warmup" })}
            onInvite={() => act({ type: "invite" })}
          />
        </div>
        <div className="player-room-pool" {...(compact && panelProps("pool"))}>
          <RoomPool
            page={page}
            me={me}
            turn={myTurn}
            pending={pending}
            onChoose={choose}
            onHold={hold}
          />
        </div>
        <div className="player-room-side" {...(compact && panelProps("room"))}>
          <section className="player-room-sheet" aria-label="Skor">
            <ScoreSheet
              room={match}
              teams={teams}
              tally={page.tally}
              finished={page.finished}
              roster={roster}
              status={<SheetStatus page={page} />}
            />
          </section>
          <RoomActions
            page={page}
            me={me}
            now={now}
            busy={busy}
            onAbort={() => act({ type: "abort" })}
            onReport={(text) => act({ type: "report", text })}
            onStop={() => act({ type: "stop" })}
          />
        </div>
        <div className="player-room-chat" {...(compact && panelProps("chat"))}>
          <MatchChat
            rooms={roomChats}
            colors={colors}
            referee={match.referee?.username ?? null}
            loaded={page.chatLoaded}
            live={page.live}
            connected={connected}
            visible={!compact || tab === "chat"}
            onUnread={setChatUnread}
          />
        </div>
      </div>
    </div>
  );
}

/**
 * The room fills the screen under whatever is above it (the header's height changes with the
 * match's names), so the page itself never scrolls on a desktop: only the pool, the score and the
 * chat do.
 */
function useRoomHeight(loaded: boolean) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const grid = ref.current;
    if (!grid) return;
    const update = () =>
      grid.style.setProperty(
        "--room-top",
        `${Math.round(grid.getBoundingClientRect().top + window.scrollY)}px`,
      );
    update();
    const observer = new ResizeObserver(update);
    if (grid.parentElement) observer.observe(grid.parentElement);
    window.addEventListener("resize", update);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", update);
    };
  }, [loaded]);
  return ref;
}

/**
 * A team's name in its side's colour, as osu! seats it (blue, slots 1-2; red, 3-4): the room page
 * wears osu!'s own blue and red, like the referee page, so it reads the same as the game. The
 * viewer's own team says "siz" after it.
 */
function SideName({ name, side, own }: { name: string; side: SlotTeam; own: boolean }) {
  return (
    <span className={`player-room-side-name is-team-${side}`}>
      {name}
      {own && <span className="player-room-own"> (siz)</span>}
    </span>
  );
}

/**
 * Who plays, under the heading: a bracket match's players in their side's colours (the heading names
 * the teams), a lobby's under each team's name. While the room is open each name carries whether
 * they're in it, as the room's bot reads it (a DEV_LOGIN dummy counts as in).
 */
function RoomRoster({ page }: { page: MatchPageState }) {
  const { match, roster, lobby, findRoster, live } = page;
  if (!match || roster.length === 0) return null;
  const bracket = match.kind === "bracket";
  const standIns = page.liveRoom?.autoref?.stand_ins === true;
  const inRoom = new Set(lobby.slots.map((s) => findRoster(s)?.player.id));
  const order = bracket
    ? [match.team1_id, match.team2_id].filter((id): id is string => id !== null)
    : match.teams.map((t) => t.id);
  const groups = order
    .map((id) => roster.filter((r) => r.team.id === id))
    .filter((players) => players.length > 0);
  return (
    <ul className="player-room-roster" aria-label="Oyuncular">
      {groups.map((players) => {
        const { team, color } = players[0];
        return (
          <li
            key={team.id}
            className={
              color ? `player-room-roster-team is-team-${color}` : "player-room-roster-team"
            }
          >
            {!bracket && <span className="player-room-roster-label">{team.name}</span>}
            {players.map((r) => {
              const present = inRoom.has(r.player.id) || (standIns && r.player.osu_id < 0);
              const where = present ? "odada" : "odada değil";
              return (
                <span
                  key={r.player.id}
                  className={`player-room-player${live ? (present ? " is-in" : " is-out") : ""}`}
                  title={live ? `${r.player.username}: ${where}` : undefined}
                >
                  {live && <span className="player-room-presence" aria-hidden="true" />}
                  {r.player.username}
                  {live && <span className="sr-only">, {where}</span>}
                </span>
              );
            })}
          </li>
        );
      })}
    </ul>
  );
}

/** The clock every countdown on the page reads, ticking twice a second while the match is on. */
function useNow(running: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!running) return;
    const timer = window.setInterval(() => setNow(Date.now()), 500);
    return () => window.clearInterval(timer);
  }, [running]);
  return now;
}

function SheetStatus({ page }: { page: MatchPageState }) {
  const { match, teams, live } = page;
  if (!match) return null;
  if (match.forfeit_slot) return <>Hükmen: {teams[match.forfeit_slot - 1]?.name} çekildi</>;
  if (match.concluded_at) return <>Maç bitti</>;
  return <>{live ? "Map bittikçe güncellenir" : "Henüz başlamadı"}</>;
}

/** The phone's pool tab: the team's own turn and its clock, otherwise how many maps. */
function PoolTabStatus({ page, me, now }: { page: MatchPageState; me: RoomMe; now: number }) {
  const autoref = page.liveRoom?.autoref;
  const mine = autoref?.enabled && autoref.team_id === me.teamId;
  if (mine && (autoref.phase === "draft" || autoref.phase === "warmup")) {
    const end = shownEnd(autoref.deadline, page.match?.starts_at ?? null);
    const what = autoref.phase === "warmup" ? "Warmup" : autoref.step === "ban" ? "Ban" : "Pick";
    return (
      <span className="player-room-tab-yours">
        {what} sizde{end !== null && <span className="num"> {clock(end - now)}</span>}
      </span>
    );
  }
  return <span className="num">{page.match ? page.match.maps.length : 0} map</span>;
}

/** The phone's match tab: the score in the sides' colours. */
function ScoreTabStatus({ page }: { page: MatchPageState }) {
  const { tally, teams } = page;
  if (!tally || teams.length !== 2) return <span>Skor</span>;
  return (
    <span className="referee-tab-score num">
      <span className="is-team-blue">{tally.team1}</span>
      <span aria-hidden="true">–</span>
      <span className="sr-only"> - </span>
      <span className="is-team-red">{tally.team2}</span>
    </span>
  );
}
