import { useEffect, useState } from "react";
import { IconButton } from "../../../components/IconButton";
import { SCORE_MODES, type ScoreMode } from "../../../lib/bancho";
import { formatLength } from "../../../lib/format";
import { RoomsEditor } from "../MatchControls";
import { MatchHistory } from "./MatchHistory";
import { AnnounceScore } from "./MatchScore";
import { RoomPlayers } from "./RoomPlayers";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./LobbyPanel.css";

/**
 * The match column, the middle of the referee's cockpit, read top to bottom as the match goes:
 * the room, the map on with its clock, the players as the room has them, the commands that act
 * on them, then the score over every map played so far, the maps it was counted from. The room's links and setup options open from the
 * room bar, since they're set once.
 */
export function LobbyPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, channel, online, inChannel, canRun, busy, actions } = referee;
  const [settings, setSettings] = useState(false);
  if (!room) return null;
  const bracket = room.kind === "bracket";
  const concluded = room.concluded_at !== null;
  const current = room.rooms.length > 0 ? room.rooms[room.rooms.length - 1] : null;
  const notOnline = online ? undefined : "Önce Bancho'ya bağlan";

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
          <span className="lobby-bar-actions">
            {channel && online && !inChannel && !concluded && (
              <button type="button" className="btn btn-secondary btn-sm" onClick={actions.rejoin}>
                Kanala katıl
              </button>
            )}
            {channel && !concluded && (
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                disabled={!online || !canRun || busy}
                title={
                  notOnline ??
                  (canRun
                    ? "Oda bozulduysa ya da kapandıysa yenisini açar; eskisi listede kalır"
                    : "Bu maçın hakemi değilsin")
                }
                onClick={() => {
                  const open = inChannel && !lobby.closed;
                  if (!open || window.confirm(`${channel} hâlâ açık görünüyor. Yine de yeni bir oda açılsın mı?`)) {
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
            {!concluded && (
              <button
                type="button"
                className="btn btn-danger btn-sm"
                disabled={!online || !inChannel || lobby.closed}
                title={inChannel ? "!mp close: odayı osu!'da kapatır" : "Önce odanın kanalına katıl"}
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
              Oda {bracket ? "Team Vs" : "Head to Head"} olarak kurulur, kilitlenir ve oyuncular davet edilir.
            </p>
            <RoomOptions referee={referee} />
            <button
              type="button"
              className="btn btn-primary"
              disabled={!online || !canRun || busy || concluded}
              title={notOnline ?? (canRun ? undefined : "Bu maçın hakemi değilsin")}
              onClick={actions.createRoom}
            >
              {busy ? "Oda kuruluyor…" : "Oda oluştur"}
            </button>
            {canRun && !concluded && (
              <div className="lobby-link">
                <span className="muted">Odayı başka yerden açtıysan linkini ekle:</span>
                <RoomsEditor rooms={[]} editable disabled={busy} onAdd={actions.addRoom} onRemove={actions.removeRoom} />
              </div>
            )}
          </div>
        )}
        {channel && <MapClock referee={referee} />}
        <RoomPlayers referee={referee} />
        {channel && !concluded && <Commands referee={referee} />}
        <MatchHistory referee={referee} />
      </div>
    </section>
  );
}

/** The match and room commands under the players, with the line that tells the room the score. */
function Commands({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, online, inChannel, actions } = referee;
  const [timer, setTimer] = useState(90);
  const [countdown, setCountdown] = useState(5);
  const ready = online && inChannel;
  const bracket = room?.kind === "bracket";

  return (
    <div className="lobby-commands" role="group" aria-label="Komutlar">
      {!ready && (
        <p className="lobby-commands-hint">
          {online ? "Komutlar, odanın kanalına katılınca açılır." : "Komutlar için önce Bancho'ya bağlan (sağda)."}
        </p>
      )}
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
          <button type="button" className="btn btn-secondary btn-sm" disabled={!ready} onClick={actions.fixTeams}>
            Takımları düzelt
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
      </div>
    </div>
  );
}

/** How the room is set up: score type, NoFail on every map, moving players to their colour. */
function RoomOptions({ referee }: { referee: RefereeRoomState }) {
  const { room, options, actions } = referee;
  return (
    <div className="lobby-options">
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
            checked={options.autoTeams}
            onChange={(e) => actions.setOptions({ ...options, autoTeams: e.target.checked })}
          />{" "}
          Girenleri takım rengine taşı
        </label>
      )}
    </div>
  );
}

/** The current time, ticking while `active`. */
function useNow(active: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    setNow(Date.now());
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, [active]);
  return now;
}

/**
 * The map the room has on and, while it's played, a countdown of its length from BanchoBot's
 * "The match has started!", so the referee knows when to expect the result. The length is the
 * pool's, already adjusted for the map's mods (DT, HT). A map outside the pool has no known
 * length, so its clock counts up instead. Under the name, the room's mods and team mode.
 */
function MapClock({ referee }: { referee: RefereeRoomState }) {
  const { lobby, currentEntry, mapStartedAt } = referee;
  const startedAt = lobby.playing ? mapStartedAt : null;
  const playing = startedAt !== null;
  const now = useNow(playing);
  const roomInfo = [lobby.mods || "Mod yok", lobby.teamMode, lobby.winCondition].filter(Boolean).join(" · ");
  if (lobby.beatmapId === null) {
    return (
      <div className="lobby-map is-empty">
        <span className="lobby-map-empty">Odada henüz map yok. Soldaki havuzdan bir map'e tıkla.</span>
        {lobby.teamMode && <span className="lobby-map-meta">{roomInfo}</span>}
      </div>
    );
  }

  const length = currentEntry?.map.total_length ?? null;
  const elapsed = startedAt !== null ? Math.max(0, Math.floor((now - startedAt) / 1000)) : 0;
  const remaining = length === null ? null : length - elapsed;
  const over = playing && remaining !== null && remaining < 0;
  const endsAt =
    startedAt !== null && length !== null
      ? new Date(startedAt + length * 1000).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" })
      : null;

  let figure: string;
  let caption: string;
  if (!playing) {
    figure = length === null ? "—" : formatLength(length);
    caption = length === null ? "havuzda değil" : "uzunluk";
  } else if (remaining === null) {
    figure = formatLength(elapsed);
    caption = "geçti · uzunluğu bilinmiyor";
  } else if (over) {
    figure = `+${formatLength(-remaining)}`;
    caption = "süre doldu, sonuç bekleniyor";
  } else {
    figure = formatLength(remaining);
    caption = `kaldı · ${endsAt} gibi biter`;
  }
  const progress = playing && length ? Math.min(1, elapsed / length) : 0;

  return (
    <div className={`lobby-map${playing ? " is-playing" : ""}${over ? " is-over" : ""}`}>
      <div className="lobby-map-text">
        <span className="lobby-map-title">
          {currentEntry && (
            <span className="lobby-map-label" style={{ color: currentEntry.color, borderColor: currentEntry.color }}>
              {currentEntry.label}
            </span>
          )}
          <a
            className="lobby-map-name"
            href={`https://osu.ppy.sh/b/${lobby.beatmapId}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            {lobby.beatmapName ?? lobby.beatmapId}
          </a>
        </span>
        <span className="lobby-map-meta">{roomInfo}</span>
      </div>
      <div className="lobby-map-clock" role="timer" aria-label={playing ? "Map süresi" : "Map uzunluğu"}>
        <span className="lobby-map-figure">{figure}</span>
        <span className="lobby-map-caption">{caption}</span>
      </div>
      {playing && length !== null && (
        <span className="lobby-map-progress" aria-hidden="true">
          <span style={{ transform: `scaleX(${progress})` }} />
        </span>
      )}
    </div>
  );
}
