import { useState, type ReactNode } from "react";
import type { Role, RoomAccess } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { ROLE_LABELS } from "../../../auth/roles";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { normalizeName, SCORE_MODES, type ScoreMode } from "../../../lib/bancho";
import { RoomsEditor } from "../MatchControls";
import { MatchHistory } from "./MatchHistory";
import { AnnounceScore } from "./MatchScore";
import { RoomPlayers } from "./RoomPlayers";
import { REF_ROLES } from "../../../lib/roomStaff";
import { botBlocker, type RefereeRoomState } from "./useRefereeRoom";
import { MapClock } from "../../../components/match/MapClock";

/**
 * The match column, the middle of the referee's cockpit, read top to bottom as the match goes:
 * the room and the bot that runs it, the map on with its clock, the players as the room has them,
 * the commands that act on them, then the score over every map played so far, the maps it was
 * counted from. The room's links, setup options and referees open from the room bar, since
 * they're seldom touched.
 */
export function LobbyPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, channel, access, botReady, canRun, busy, actions } = referee;
  const [settings, setSettings] = useState(false);
  if (!room) return null;
  const bracket = room.kind === "bracket";
  const concluded = room.concluded_at !== null;
  const current = room.rooms.length > 0 ? room.rooms[room.rooms.length - 1] : null;
  const manage = canRun ? undefined : "Bu maçın hakemi değilsin";

  return (
    <section className="lobby-panel" aria-label="Maç">
      <header className="lobby-header">
        <div className="lobby-bar">
          {channel ? (
            <h2 className="lobby-title">
              <a href={current?.url} target="_blank" rel="noopener noreferrer" title="osu!'da maç sayfası">
                {channel}
              </a>
              {room.rooms.length > 1 && <span className="lobby-title-count">{room.rooms.length}. oda</span>}
            </h2>
          ) : (
            <h2 className="lobby-title is-empty">Oda yok</h2>
          )}
          {lobby.playing && <span className="badge badge-success">Oynanıyor</span>}
          {lobby.locked && <span className="badge">Kilitli</span>}
          {lobby.closed && <span className="badge badge-danger">Kapandı</span>}
          {channel && !concluded && <BotBadge bot={referee.botName} access={access} />}
          <span className="lobby-bar-actions">
            {channel && !concluded && (
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                disabled={!canRun || busy}
                title={manage ?? "Oda bozulduysa ya da kapandıysa en az meşgul botla yenisini açar; eskisi listede kalır"}
                onClick={() => {
                  const open = access === "in" && !lobby.closed;
                  if (!open || window.confirm(`${channel} hâlâ açık. Yine de yeni bir oda açılsın mı?`)) {
                    actions.createRoom();
                  }
                }}
              >
                {busy ? "Oda kuruluyor…" : "Yeni oda aç"}
              </button>
            )}
            {channel && (
              <IconButton
                icon="settings"
                label="Odalar ve ayarlar"
                active={settings}
                aria-expanded={settings}
                onClick={() => setSettings((s) => !s)}
              />
            )}
          </span>
        </div>
      </header>

      <div className="lobby-body">
        {channel && !concluded && !botReady && canRun && <BotNotice referee={referee} />}
        {channel && settings && (
          <div className="lobby-settings">
            <RoomsEditor
              rooms={room.rooms}
              editable={canRun && !concluded}
              disabled={busy}
              onAdd={actions.addRoom}
              onRemove={actions.removeRoom}
            />
            <RoomOptions referee={referee} />
            {!concluded && <RoomReferees referee={referee} />}
            {!concluded && (
              <button
                type="button"
                className="btn btn-danger btn-sm"
                disabled={!botReady || lobby.closed}
                title={botBlocker(referee) ?? "!mp close: odayı osu!'da kapatır"}
                onClick={() => {
                  if (window.confirm("Oda kapatılsın mı (!mp close)? Bu geri alınamaz.")) actions.close();
                }}
              >
                Odayı kapat
              </button>
            )}
          </div>
        )}
        {!channel && (
          <div className="lobby-create">
            <p className="lobby-create-text">
              Oda, en az meşgul botla açılır: {bracket ? "Team Vs" : "Head to Head"} olarak kurulur, kilitlenir ve
              oyuncular davet edilir. Bancho'ya bağlanman gerekmez; bağlanırsan bot seni odanın hakemi yapar.
            </p>
            <RoomOptions referee={referee} />
            <button
              type="button"
              className="btn btn-primary"
              disabled={!canRun || busy || concluded}
              title={manage}
              onClick={actions.createRoom}
            >
              {busy ? "Oda kuruluyor…" : "Oda oluştur"}
            </button>
            {canRun && !concluded && (
              <div className="lobby-link">
                <span className="muted">
                  Odayı başka yerden açtıysan linkini ekle; bir bot devralır, odayı açan kişi onu{" "}
                  <code>!mp addref</code> ile hakem yapınca odaya girer.
                </span>
                <RoomsEditor rooms={[]} editable disabled={busy} onAdd={actions.addRoom} onRemove={actions.removeRoom} />
              </div>
            )}
          </div>
        )}
        {channel && (
          <MapClock
            lobby={referee.lobby}
            currentEntry={referee.currentEntry}
            mapStartedAt={referee.mapStartedAt}
            emptyText="Odada henüz map yok. Soldaki havuzdan bir map'e tıkla."
          />
        )}
        <RoomPlayers referee={referee} />
        {channel && !concluded && <Commands referee={referee} />}
        <MatchHistory referee={referee} />
      </div>
    </section>
  );
}

/** The match and room commands under the players, with the line that tells the room the score. The room's bot sends them. */
function Commands({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, botReady, access, actions } = referee;
  const [timer, setTimer] = useState(90);
  const [countdown, setCountdown] = useState(5);
  const ready = botReady;
  const bracket = room?.kind === "bracket";
  const blocker = botBlocker(referee);

  return (
    <div className="lobby-commands" role="group" aria-label="Komutlar">
      {/* A bot that can't get in says why above the column; one on its way in, here. */}
      {!ready && access === "joining" && <p className="lobby-commands-hint">{blocker} Komutlar o girince açılır.</p>}
      <div className="lobby-command-row" role="group" aria-label="Maç komutları">
        <div className="lobby-command-group">
          <input
            className="input lobby-seconds"
            type="number"
            min={0}
            aria-label="Başlatma geri sayımı (saniye)"
            value={countdown}
            onChange={(e) => setCountdown(Number(e.target.value))}
          />
          <button
            type="button"
            className="btn btn-primary lobby-start"
            disabled={!ready || lobby.playing}
            onClick={() => actions.start(countdown)}
          >
            Başlat
          </button>
          <button
            type="button"
            className="btn btn-danger"
            disabled={!ready || !lobby.playing}
            onClick={() => {
              if (window.confirm("Map iptal edilsin mi (!mp abort)?")) actions.abort();
            }}
          >
            İptal et
          </button>
        </div>
        <div className="lobby-command-group">
          <input
            className="input lobby-seconds"
            type="number"
            min={0}
            aria-label="Zamanlayıcı süresi (saniye)"
            value={timer}
            onChange={(e) => setTimer(Number(e.target.value))}
          />
          <button type="button" className="btn btn-secondary" disabled={!ready} onClick={() => actions.timer(timer)}>
            Zamanlayıcı
          </button>
          <button type="button" className="btn btn-secondary" disabled={!ready} onClick={actions.abortTimer}>
            Durdur
          </button>
        </div>
      </div>
      {bracket && <AnnounceScore referee={referee} />}
      <div className="lobby-command-row is-room" role="group" aria-label="Oda komutları">
        <button type="button" className="btn btn-secondary btn-sm" disabled={!ready} onClick={actions.settings}>
          Odayı yenile
        </button>
        <button type="button" className="btn btn-secondary btn-sm" disabled={!ready} onClick={actions.inviteAll}>
          Hepsini davet et
        </button>
        {bracket && (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={!ready}
            title="Mavi takımı 1-2. slotlara, kırmızı takımı 3-4. slotlara ve kendi rengine taşır"
            onClick={actions.fixPlacement}
          >
            Yerleri düzelt
          </button>
        )}
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={!ready}
          onClick={() => actions.lock(!lobby.locked)}
        >
          {lobby.locked ? "Kilidi aç" : "Kilitle"}
        </button>
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={!ready}
          title="Takım modu, skor türü, boyut, kilit ve davetleri yeniden gönderir"
          onClick={actions.setUpRoom}
        >
          Kurulumu tekrarla
        </button>
        <StreamerRef referee={referee} ready={ready} blocker={blocker} />
      </div>
    </div>
  );
}

/**
 * Makes the match's streamer a referee of the room (!mp addref), so their tournament client shows
 * the room's chat on stream; pressed once the room's chat says they are, when it takes it back.
 */
function StreamerRef({ referee, ready, blocker }: { referee: RefereeRoomState; ready: boolean; blocker: string | null }) {
  const { streamer, streamerIsRef, actions } = referee;
  if (!streamer) return null;
  return (
    <button
      type="button"
      className={`btn btn-secondary btn-sm lobby-streamer-ref${streamerIsRef ? " is-on" : ""}`}
      aria-pressed={streamerIsRef}
      disabled={!ready}
      title={
        !ready
          ? (blocker ?? undefined)
          : streamerIsRef
            ? `${streamer.username} odanın hakemi. Tıklarsan hakemlikten çıkar (!mp removeref).`
            : `${streamer.username} odanın hakemi olur (!mp addref): turnuva istemcisinde oda sohbetini görür.`
      }
      onClick={actions.toggleStreamerRef}
    >
      <Icon name={streamerIsRef ? "check" : "monitor"} size={14} />
      {streamerIsRef ? (
        <>
          Yayıncı hakem <span className="lobby-streamer-name">{streamer.username}</span>
        </>
      ) : (
        <>
          Yayıncıyı hakem ekle <span className="lobby-streamer-name">{streamer.username}</span>
        </>
      )}
    </button>
  );
}

const ACCESS_LABELS: Record<RoomAccess, string> = {
  in: "odada",
  joining: "odaya giriyor",
  refused: "odaya giremiyor",
  offline: "bağlı değil",
  no_bot: "bot yok",
  closed: "oda kapandı",
};

/**
 * The bot that runs the room, and where it stands: in the room (it logs and runs it), on its way,
 * or kept out. Only a bot in the room takes commands.
 */
function BotBadge({ bot, access }: { bot: string | null; access: RoomAccess | null }) {
  if (!access) return null;
  const tone = access === "in" ? "is-in" : access === "joining" ? "is-joining" : "is-out";
  return (
    <span className={`lobby-bot ${tone}`} title={bot ? `Odayı ${bot} yönetiyor: ${ACCESS_LABELS[access]}` : undefined}>
      <span className="lobby-bot-dot" aria-hidden="true" />
      <Icon name="bot" size={14} />
      <span className="lobby-bot-name">{bot ?? "Bot yok"}</span>
      {access !== "in" && bot && <span className="lobby-bot-state">{ACCESS_LABELS[access]}</span>}
      {access === "in" && <span className="sr-only">, odada</span>}
    </span>
  );
}

/**
 * Why the bot can't run the room, and the way out: a room made elsewhere waits for its maker to
 * add the bot, and a room whose bot dropped moves to a new room with another bot (bots never
 * join each other's rooms: Bancho counts that against their limit).
 */
function BotNotice({ referee }: { referee: RefereeRoomState }) {
  const { access, botName, busy, actions } = referee;
  if (access === "joining" || access === null || access === "in") return null;
  const bot = botName ?? "bot";
  let title: string;
  let body: ReactNode;
  let action: ReactNode = (
    <button type="button" className="btn btn-primary btn-sm" disabled={busy} onClick={actions.createRoom}>
      {busy ? "Oda kuruluyor…" : "Başka botla yeni oda aç"}
    </button>
  );
  switch (access) {
    case "refused":
      title = `${bot} odaya giremiyor`;
      body = (
        <>
          Oda başka yerden açıldı. Açan kişi odada <code>!mp addref {bot}</code> yazınca bot girer; o zamana kadar ne
          komut gönderebilir ne de sohbeti kaydedebilir.
        </>
      );
      action = (
        <button type="button" className="btn btn-secondary btn-sm" onClick={actions.rejoin}>
          Tekrar dene
        </button>
      );
      break;
    case "offline":
      title = `${bot} Bancho'ya bağlı değil`;
      body = <>Bağlanınca odaya döner. Beklemeden sürdürmek için maçı başka bir botun açacağı yeni odaya taşı.</>;
      break;
    case "no_bot":
      title = "Bu odayı bir bot yönetmiyor";
      body = <>Komutlar ve sohbet kaydı bot ister. Maçı bir botun açacağı yeni odaya taşı.</>;
      break;
    case "closed":
      title = "Oda kapandı";
      body = <>Maç sürüyorsa yeni bir oda aç; eski oda sohbetiyle listede kalır.</>;
      break;
  }
  return (
    <div className="lobby-notice" role="status">
      <p className="lobby-notice-text">
        <strong className="lobby-notice-title">{title}</strong>
        <span>{body}</span>
      </p>
      {action}
    </div>
  );
}

/**
 * The room's referees as Bancho has them (!mp addref). A referee who opens the match with Bancho
 * connected is added by the bot and taken off two minutes after they leave; anyone added here, and
 * the streamer, stays until taken off. Referees, streamers and commentators (for the tournament
 * client's chat) can be added, each named with their role; never a bot or a player of the match.
 */
function RoomReferees({ referee }: { referee: RefereeRoomState }) {
  const { lobby, staff, roster, botReady, actions } = referee;
  const me = useSignedInUser();
  const [adding, setAdding] = useState("");
  const blocker = botBlocker(referee);
  const nameOf = (ref: string) => staff?.find((s) => normalizeName(s.username) === ref)?.username ?? ref;
  const candidates = (staff ?? []).filter(
    (s) =>
      REF_ROLES.some((r) => s.roles.includes(r)) &&
      !lobby.refs.includes(normalizeName(s.username)) &&
      !roster.some((p) => p.player.osu_id === s.osu_id),
  );
  // The role they'd be there as, first of REF_ROLES they hold.
  const roleOf = (roles: Role[]) => ROLE_LABELS[REF_ROLES.find((r) => roles.includes(r)) ?? "referee"];

  return (
    <section className="lobby-refs" aria-labelledby="lobby-refs-title">
      <h3 id="lobby-refs-title" className="lobby-refs-title">
        Odanın hakemleri <span className="lobby-refs-count num">{lobby.refs.length}</span>
      </h3>
      <p className="lobby-refs-hint">
        Maçı Bancho bağlıyken açan hakemleri bot ekler, ayrıldıktan iki dakika sonra çıkarır.
      </p>
      {lobby.refs.length > 0 && (
        <ul className="lobby-refs-list">
          {lobby.refs.map((ref) => {
            const name = nameOf(ref);
            return (
              <li key={ref} className="lobby-refs-row">
                <span className="lobby-refs-name">
                  {name}
                  {normalizeName(me.username) === ref && <span className="lobby-refs-self"> (sen)</span>}
                </span>
                <IconButton
                  icon="close"
                  iconSize={14}
                  label={`${name} hakemlikten çıkarılsın (!mp removeref)`}
                  disabled={!botReady}
                  onClick={() => actions.removeRef(name)}
                />
              </li>
            );
          })}
        </ul>
      )}
      <form
        className="lobby-refs-add"
        onSubmit={(e) => {
          e.preventDefault();
          if (!adding) return;
          actions.addRef(adding);
          setAdding("");
        }}
      >
        <select
          className="select select-sm"
          aria-label="Hakem ekle"
          value={adding}
          disabled={!botReady || candidates.length === 0}
          onChange={(e) => setAdding(e.target.value)}
        >
          <option value="">{candidates.length === 0 ? "Eklenecek kimse yok" : "Hakem, yayıncı ya da yorumcu seç"}</option>
          {candidates.map((s) => (
            <option key={s.id} value={s.username}>
              {s.username} · {roleOf(s.roles)}
            </option>
          ))}
        </select>
        <button
          type="submit"
          className="btn btn-secondary btn-sm"
          disabled={!botReady || !adding}
          title={blocker ?? "!mp addref: turnuva istemcisinde odanın sohbetini görür ve komut yazabilir"}
        >
          Ekle
        </button>
      </form>
    </section>
  );
}

/** How the room is set up: score type, NoFail on every map, moving players to their colour. */
function RoomOptions({ referee }: { referee: RefereeRoomState }) {
  const { room, options, canRun, actions } = referee;
  // Shared by everyone on the page, so only those who run the match change them.
  const locked = !canRun || room?.concluded_at != null;
  return (
    <fieldset className="lobby-options" disabled={locked}>
      <label>
        Skor türü{" "}
        <select
          className="select select-sm"
          value={options.scoreMode}
          onChange={(e) => actions.setOptions({ ...options, scoreMode: Number(e.target.value) as ScoreMode })}
        >
          {SCORE_MODES.map((m) => (
            <option key={m.value} value={m.value}>
              {m.label}
            </option>
          ))}
        </select>
      </label>
      <label>
        <input
          type="checkbox"
          checked={options.noFail}
          onChange={(e) => actions.setOptions({ ...options, noFail: e.target.checked })}
        />{" "}
        Haritalara NF ekle
      </label>
      {room?.kind === "bracket" && (
        <label>
          <input
            type="checkbox"
            checked={options.autoPlace}
            onChange={(e) => actions.setOptions({ ...options, autoPlace: e.target.checked })}
          />{" "}
          Girenleri yerine ve takım rengine taşı
        </label>
      )}
      <label title="Maçın oyuncusu, host, hakem ya da yayıncı olmayan herkes girer girmez atılır">
        <input
          type="checkbox"
          checked={options.autoKick}
          onChange={(e) => actions.setOptions({ ...options, autoKick: e.target.checked })}
        />{" "}
        Maç dışından girenleri at
      </label>
    </fieldset>
  );
}
