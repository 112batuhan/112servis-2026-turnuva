import { useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { Countdown } from "../../../components/Countdown";
import { DiscordMention } from "../../../components/DiscordMention";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { LiveMark } from "../../../components/LiveMark";
import { TeamCard } from "../../../components/TeamCard";
import { normalizeName, type SlotTeam } from "../../../lib/bancho";
import { stageLabel } from "../../../lib/bestOf";
import { bracketMatchPath, isOwnMatch, roomPagePath } from "../../../lib/bracket";
import { useAuth } from "../../../auth/AuthContext";
import { Icon } from "../../../components/Icon";
import { formatMatchTime } from "../../../lib/format";
import { MapClock } from "../../../components/match/MapClock";
import { ScoreSheet } from "../../../components/match/ScoreSheet";
import { poolEntries } from "../../../lib/matchPool";
import { draftPhase, PoolView, TurnStatus } from "../../../components/match/PoolView";
import { CockpitTabs, panelProps, TABS, useCockpit } from "../../../components/match/Cockpit";
import { PlayersView } from "../../../components/match/PlayersView";
import { staffRole, useRoomStaff } from "../../../lib/roomStaff";
import { roomLabel } from "../../../lib/liveRoom";
import { MatchChat } from "./MatchChat";
import { MatchPickem } from "./MatchPickem";
import { MatchSpectators } from "./MatchSpectators";
import { useMatchPage, type MatchPageState } from "../../../lib/useMatchPage";
import "../../../components/match/Cockpit.css";
import "../../../components/match/MapClock.css";
import "../../../components/match/ScoreSheet.css";
import "../../../components/match/PoolView.css";
import "../../../components/match/PlayersView.css";
import "./MatchPage.css";

/**
 * /match/:matchId — a match as its referee runs it, for everyone, live: the pool with the draft,
 * the map on with its clock, the players in their slots with their mods, the score sheet and the
 * room's chat. The referee page's cockpit without anything that runs the room.
 */
export function MatchPage() {
  const { matchId = "" } = useParams();
  return <MatchView key={matchId} matchId={matchId} />;
}

function MatchView({ matchId }: { matchId: string }) {
  const page = useMatchPage(matchId);
  const { match, loadError, roomChats, roster, connected } = page;
  const { tab, select, compact, pageRef, barRef, tabsRef } = useCockpit(match !== null);
  const [chatUnread, setChatUnread] = useState(0);
  const { user } = useAuth();

  const colors = useMemo(() => {
    const map = new Map<string, SlotTeam | null>();
    for (const r of roster) map.set(normalizeName(r.player.username), r.color);
    return map;
  }, [roster]);

  if (!match) {
    if (loadError) {
      return (
        <div className="page referee-page match-page">
          <ErrorAlert message={loadError} />
          <Link to="/bracket">← Bracket</Link>
        </div>
      );
    }
    return <p className="status">Yükleniyor…</p>;
  }

  const bracket = match.kind === "bracket";
  const teamName = (id: string | null) =>
    match.teams.find((t) => t.id === id)?.name ?? "Belirlenmedi";
  // With both teams known, the score sheet names them.
  const named = page.tally !== null && page.teams.length === 2;

  return (
    <div className="page referee-page match-page" ref={pageRef}>
      <header className="referee-header match-header">
        <div className="match-header-lead">
          <Link
            to={bracket ? bracketMatchPath(match.id, false) : "/qualifiers"}
            className="referee-back"
          >
            {bracket ? "← Bracket" : "← Elemeler"}
          </Link>
          <h1 className="referee-title">
            <span className={match.side ? `match-code is-${match.side}` : undefined}>
              {roomLabel(match)}
            </span>{" "}
            {bracket ? (
              <span className={named ? "sr-only" : "referee-title-teams"}>
                {teamName(match.team1_id)} vs {teamName(match.team2_id)}
              </span>
            ) : (
              <span className="referee-title-teams">· {match.teams.length} takım</span>
            )}
          </h1>
          {isOwnMatch(
            user?.team_id,
            match.teams.map((t) => t.id),
            match.concluded_at !== null,
          ) && (
            <Link to={roomPagePath(match.id)} className="btn btn-primary btn-sm match-room-link">
              <Icon name="swords" size={14} />
              Maç odası
            </Link>
          )}
        </div>
        <p className="referee-meta match-meta">
          {match.starts_at && (
            <time className="match-meta-time" dateTime={match.starts_at}>
              {formatMatchTime(match.starts_at)}
            </time>
          )}
          {match.stage && (
            <span className="match-meta-value">
              {match.kind === "bracket" ? stageLabel(match.stage) : match.stage.name}
            </span>
          )}
          <span>
            Hakem{" "}
            {match.referee ? (
              <>
                <span className="match-meta-value">{match.referee.username}</span>{" "}
                <DiscordMention player={match.referee} fallback="" />
              </>
            ) : (
              "henüz yok"
            )}
          </span>
          {bracket && (
            <span>
              Yayıncı{" "}
              {match.streamer ? (
                <span className="match-meta-value">{match.streamer.username}</span>
              ) : (
                "henüz yok"
              )}
            </span>
          )}
        </p>
        {page.live ? (
          <div className="match-onair">
            <LiveMark large />
            {page.spectators && <MatchSpectators spectators={page.spectators} />}
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
          label="Maç paneli"
          active={tab}
          onSelect={select}
          status={{
            pool: <PoolStatus page={page} />,
            room: <RoomStatus page={page} />,
            chat: (
              <ChatStatus
                unread={chatUnread}
                live={page.live}
                connected={connected}
                lines={roomChats.length > 0 ? roomChats[roomChats.length - 1].lines.length : 0}
              />
            ),
          }}
        />
        <ErrorAlert message={loadError} />
      </div>

      <div className="referee-cockpit" data-tab={tab}>
        {TABS.map(({ id }) => (
          <div key={id} className={`referee-cockpit-${id}`} {...(compact && panelProps(id))}>
            {id === "pool" && (
              <PoolView
                room={match}
                currentEntry={match.concluded_at ? undefined : page.currentEntry}
                played={page.played}
                draft={page.draft}
                phase={
                  page.live && page.teams.length === 2 ? draftPhase(page.turn, page.standing) : null
                }
              />
            )}
            {id === "room" && <MatchColumn page={page} />}
            {id === "chat" && (
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
            )}
          </div>
        ))}
      </div>
    </div>
  );
}

/**
 * The middle column, as on the referee page without its controls: the room, the map on with its
 * clock, the players in their slots, then the score over every map played.
 */
function MatchColumn({ page }: { page: MatchPageState }) {
  const { match, lobby, roster, findRoster, teams, tally, finished } = page;
  // Staff in the room are named by their role rather than left as strangers.
  const staff = useRoomStaff();
  if (!match) return null;
  const current = match.rooms.length > 0 ? match.rooms[match.rooms.length - 1] : null;

  return (
    <section className="lobby-panel" aria-label="Maç">
      <header className="lobby-header">
        <div className="lobby-bar">
          {current ? (
            <h2 className="lobby-title">
              <a
                href={current.url}
                target="_blank"
                rel="noopener noreferrer"
                title="osu!'da maç sayfası"
              >
                #mp_{current.mp_id}
              </a>
              {match.rooms.length > 1 && (
                <span className="lobby-title-count">{match.rooms.length}. oda</span>
              )}
            </h2>
          ) : (
            <h2 className="lobby-title is-empty">Oda henüz açılmadı</h2>
          )}
          {lobby.playing && <span className="badge badge-success">Oynanıyor</span>}
          {lobby.closed && <span className="badge badge-danger">Kapandı</span>}
          {current && (
            <a
              className="btn btn-secondary btn-sm match-osu-link"
              href={current.url}
              target="_blank"
              rel="noopener noreferrer"
            >
              osu!'da aç
            </a>
          )}
        </div>
      </header>

      <div className="lobby-body">
        {/* A finished match's room says nothing more: the score sheet is its record. */}
        {current ? (
          !match.concluded_at && (
            <MapClock
              lobby={lobby}
              currentEntry={page.currentEntry}
              mapStartedAt={page.mapStartedAt}
              emptyText="Odada henüz map yok. Hakem bir map seçince burada görünür."
            />
          )
        ) : (
          <Waiting page={page} />
        )}
        {match.concluded_at ? (
          <MatchTeamCards page={page} />
        ) : (
          <PlayersView
            kind={match.kind}
            teams={match.teams}
            roster={roster}
            lobby={lobby}
            findRoster={findRoster}
            staffRoleOf={(slot) => staffRole(staff, slot)}
            banners
          />
        )}
        <ScoreSheet
          room={match}
          teams={teams}
          tally={tally}
          finished={finished}
          roster={roster}
          status={<SheetStatus page={page} />}
        />
        <MatchPickem match={match} />
      </div>
    </section>
  );
}

/**
 * A finished match's teams as the teams page shows them: each one's card, ruled in its side's
 * colour (blue then red in a bracket match), since the room's state no longer says anything.
 */
function MatchTeamCards({ page }: { page: MatchPageState }) {
  const { match, roster } = page;
  if (!match) return null;
  if (match.teams.length === 0) return <p className="muted">Bu maçta takım yok.</p>;
  const colorOf = (id: string) => roster.find((r) => r.team.id === id)?.color ?? null;
  return (
    <section className="team-grid is-ruled match-team-grid" aria-label="Takımlar">
      {match.teams.map((team) => {
        const color = colorOf(team.id);
        return (
          <div key={team.id} className={`match-team-side${color ? ` is-team-${color}` : ""}`}>
            <TeamCard team={team} />
          </div>
        );
      })}
    </section>
  );
}

/** Before the room is open: when the match starts, and what will show up here. */
function Waiting({ page }: { page: MatchPageState }) {
  const { match } = page;
  if (!match) return null;
  if (match.concluded_at) {
    return (
      <div className="match-waiting">
        <p className="match-waiting-title">Maç oda açılmadan sonuçlandı.</p>
      </div>
    );
  }
  const future = match.starts_at !== null && new Date(match.starts_at).getTime() > Date.now();
  return (
    <div className="match-waiting">
      <p className="match-waiting-title">
        {match.starts_at ? (
          <>
            <time dateTime={match.starts_at}>{formatMatchTime(match.starts_at)}</time>
            {future && (
              <Countdown to={match.starts_at} suffix="kaldı" className="match-waiting-countdown" />
            )}
          </>
        ) : (
          "Maçın saati henüz belli değil"
        )}
      </p>
      <p className="match-waiting-text">
        Hakem odayı açınca oyuncular, oynanan map ve skor burada canlı görünür; sayfayı yenilemen
        gerekmez.
      </p>
    </div>
  );
}

/** Under the score sheet's title: whether the score is still moving. */
function SheetStatus({ page }: { page: MatchPageState }) {
  const { match, teams, live } = page;
  if (!match) return null;
  if (match.forfeit_slot) return <>Hükmen: {teams[match.forfeit_slot - 1]?.name} çekildi</>;
  if (match.concluded_at) return <>Maç bitti</>;
  if (live) {
    return <span title="Odanın botu map bittikçe skoru kaydeder">Map bittikçe güncellenir</span>;
  }
  return <>Henüz başlamadı</>;
}

/** The phone's pool tab: the next ban or pick by the stage's order, its team in their colour. */
function PoolStatus({ page }: { page: MatchPageState }) {
  const { match, teams, turn, live, standing } = page;
  if (!match) return null;
  const maps = poolEntries(match).length;
  if (!live || teams.length !== 2 || maps === 0 || standing?.kind === "won")
    return <span className="num">{maps} map</span>;
  if (standing?.kind === "tiebreaker") return <span>Tiebreaker</span>;
  if (!turn) return <span className="num">{maps} map</span>;
  return <TurnStatus turn={turn} />;
}

/** The phone's match tab: the score in the sides' colours, lit while a map plays. */
function RoomStatus({ page }: { page: MatchPageState }) {
  const { match, tally, teams, lobby } = page;
  if (!match) return null;
  const playing = lobby.playing && !match.concluded_at;
  let text;
  if (tally && teams.length === 2 && (match.rooms.length > 0 || match.concluded_at)) {
    text = (
      <span className="referee-tab-score num">
        <span className="is-team-blue">
          {match.forfeit_slot === 1 ? "FF" : match.forfeit_slot === 2 ? "W" : tally.team1}
        </span>
        <span aria-hidden="true">–</span>
        <span className="sr-only"> - </span>
        <span className="is-team-red">
          {match.forfeit_slot === 2 ? "FF" : match.forfeit_slot === 1 ? "W" : tally.team2}
        </span>
      </span>
    );
  } else if (match.rooms.length === 0) {
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
      {playing && <span className="referee-tab-live" title="Map oynanıyor" />}
      {playing && <span className="sr-only">Map oynanıyor, </span>}
      {text}
    </>
  );
}

/**
 * The phone's chat tab: new lines waiting, or how many there are (the header already says the match
 * is live), or that it's reconnecting.
 */
function ChatStatus({
  unread,
  live,
  connected,
  lines,
}: {
  unread: number;
  live: boolean;
  connected: boolean;
  lines: number;
}) {
  if (unread > 0) return <span className="referee-tab-unread num">{unread} yeni</span>;
  if (!live) return <span>Kayıtlı</span>;
  if (!connected) return <span>Bağlanıyor…</span>;
  return (
    <span>
      <span className="num">{lines}</span> mesaj
    </span>
  );
}
