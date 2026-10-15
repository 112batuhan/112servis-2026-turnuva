import { useState } from "react";
import { IconButton } from "../../../components/IconButton";
import { SCORE_MODES, sameName, type RoomSlot, type ScoreMode } from "../../../lib/bancho";
import { RoomsEditor } from "../MatchControls";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./LobbyPanel.css";

const STATUS_LABELS: Record<RoomSlot["status"], string> = {
  ready: "Hazır",
  not_ready: "Hazır değil",
  no_map: "Map yok",
};

/** The osu! room: setup and match commands, and its slots as osu! shows them. */
export function LobbyPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, channel, online, inChannel, canRun, busy, options, actions } = referee;
  const [timer, setTimer] = useState(90);
  const [countdown, setCountdown] = useState(5);
  if (!room) return null;
  const bracket = room.kind === "bracket";
  const concluded = room.concluded_at !== null;
  const current = room.rooms.length > 0 ? room.rooms[room.rooms.length - 1] : null;
  const ready = online && inChannel;
  const notOnline = online ? undefined : "Önce Bancho'ya bağlan";

  const optionControls = (
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
      {bracket && (
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

  if (!channel) {
    return (
      <section className="lobby-panel" aria-label="osu! odası">
        <h2 className="lobby-title">osu! odası</h2>
        <p className="muted">
          Bu maçın henüz odası yok. Oda {bracket ? "Team Vs" : "Head to Head"} olarak kurulur, kilitlenir ve
          oyuncular davet edilir.
        </p>
        {optionControls}
        <div className="lobby-actions">
          <button
            type="button"
            className="btn btn-primary"
            disabled={!online || !canRun || busy || concluded}
            title={notOnline ?? (canRun ? undefined : "Bu maçın hakemi değilsin")}
            onClick={actions.createRoom}
          >
            {busy ? "Oda kuruluyor…" : "Oda oluştur"}
          </button>
        </div>
        {canRun && !concluded && (
          <div className="lobby-link">
            <span className="muted">Odayı başka yerden açtıysan linkini ekle:</span>
            <RoomsEditor rooms={[]} editable disabled={busy} onAdd={actions.addRoom} onRemove={actions.removeRoom} />
          </div>
        )}
      </section>
    );
  }

  const size = Math.max(lobby.size ?? 0, ...lobby.slots.map((s) => s.slot), referee.roster.length);
  const slots = Array.from({ length: Math.min(16, Math.max(size, 1)) }, (_, i) => lobby.slots.find((s) => s.slot === i + 1));

  return (
    <section className="lobby-panel" aria-label="osu! odası">
      <header className="lobby-header">
        <h2 className="lobby-title">
          <a href={current?.url} target="_blank" rel="noopener noreferrer">
            {channel}
          </a>
          {room.rooms.length > 1 && <span className="muted"> · {room.rooms.length}. oda</span>}
        </h2>
        {lobby.playing && <span className="badge badge-success">Oynanıyor</span>}
        {lobby.locked && <span className="badge">Kilitli</span>}
        {lobby.closed && <span className="badge badge-danger">Kapandı</span>}
        {online && !inChannel && !concluded && (
          <button type="button" className="btn btn-secondary btn-sm" onClick={actions.rejoin}>
            Kanala katıl
          </button>
        )}
        {!concluded && (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={!online || !canRun || busy}
            title={notOnline ?? (canRun ? "Oda bozulduysa ya da kapandıysa yenisini açar; eskisi listede kalır" : "Bu maçın hakemi değilsin")}
            onClick={() => {
              const open = inChannel && !lobby.closed;
              if (!open || window.confirm(`${channel} hâlâ açık görünüyor. Yine de yeni bir oda açılsın mı?`)) actions.createRoom();
            }}
          >
            {busy ? "Oda kuruluyor…" : "Yeni oda aç"}
          </button>
        )}
      </header>

      <div className="lobby-rooms">
        <span className="lobby-rooms-label">{room.rooms.length > 1 ? "Odalar" : "Oda"}</span>
        <RoomsEditor
          rooms={room.rooms}
          editable={canRun && !concluded}
          disabled={busy}
          onAdd={actions.addRoom}
          onRemove={actions.removeRoom}
        />
      </div>

      <dl className="lobby-info">
        <div>
          <dt>Map</dt>
          <dd>
            {lobby.beatmapId ? (
              <a href={`https://osu.ppy.sh/b/${lobby.beatmapId}`} target="_blank" rel="noopener noreferrer">
                {lobby.beatmapName ?? lobby.beatmapId}
              </a>
            ) : (
              <span className="muted">—</span>
            )}
          </dd>
        </div>
        <div>
          <dt>Modlar</dt>
          <dd>{lobby.mods || <span className="muted">Yok</span>}</dd>
        </div>
        <div>
          <dt>Mod</dt>
          <dd>
            {lobby.teamMode ?? "—"}
            {lobby.winCondition && ` · ${lobby.winCondition}`}
          </dd>
        </div>
      </dl>

      <ol className="lobby-slots" aria-label="Slotlar">
        {slots.map((slot, i) => (
          <SlotRow key={i} number={i + 1} slot={slot} referee={referee} />
        ))}
      </ol>

      {!ready && (
        <p className="lobby-actions-hint">
          {online ? "Komutlar, odanın kanalına katılınca açılır." : "Komutlar için önce Bancho'ya bağlan."}
        </p>
      )}
      <div className="lobby-actions is-match" role="group" aria-label="Maç komutları">
        <span className="lobby-actions-label">Maç</span>
        <div className="lobby-action-group">
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
            className="btn btn-danger btn-sm"
            disabled={!ready || !lobby.playing}
            onClick={() => {
              if (window.confirm("Map iptal edilsin mi (!mp abort)?")) actions.abort();
            }}
          >
            İptal et
          </button>
        </div>
        <div className="lobby-action-group">
          <input
            className="input lobby-seconds"
            type="number"
            min={0}
            aria-label="Zamanlayıcı süresi (saniye)"
            value={timer}
            onChange={(e) => setTimer(Number(e.target.value))}
          />
          <button type="button" className="btn btn-secondary btn-sm" disabled={!ready} onClick={() => actions.timer(timer)}>
            Zamanlayıcı
          </button>
          <button type="button" className="btn btn-secondary btn-sm" disabled={!ready} onClick={actions.abortTimer}>
            Durdur
          </button>
        </div>
      </div>

      <div className="lobby-actions" role="group" aria-label="Oda komutları">
        <span className="lobby-actions-label">Oda</span>
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
        <button
          type="button"
          className="btn btn-danger btn-sm lobby-close"
          disabled={!ready}
          onClick={() => {
            if (window.confirm("Oda kapatılsın mı (!mp close)? Bu geri alınamaz.")) actions.close();
          }}
        >
          Odayı kapat
        </button>
      </div>
      {optionControls}
    </section>
  );
}

function SlotRow({ number, slot, referee }: { number: number; slot: RoomSlot | undefined; referee: RefereeRoomState }) {
  const { lobby, actions, inChannel, online } = referee;
  if (!slot) {
    return (
      <li className="lobby-slot is-empty">
        <span className="lobby-slot-number">{number}</span>
        <span className="muted">Boş</span>
      </li>
    );
  }
  const r = referee.findRoster(slot);
  const score = lobby.current.find((s) => sameName(s.username, slot.username));
  const wrongTeam = r?.color && slot.team && r.color !== slot.team;
  const status = lobby.playing ? (score ? score.score.toLocaleString("tr-TR") : "Oynuyor") : STATUS_LABELS[slot.status];
  return (
    <li className={`lobby-slot ${slot.team ? `is-team-${slot.team}` : ""} is-${slot.status}`}>
      <span className="lobby-slot-number">{number}</span>
      <span className="lobby-slot-name">
        <a href={`https://osu.ppy.sh/users/${r?.player.osu_id ?? slot.userId ?? ""}`} target="_blank" rel="noopener noreferrer">
          {slot.username}
        </a>
        {slot.host && <span className="badge">Host</span>}
        {r ? (
          <span className="lobby-slot-team" title="Takımı">
            {r.team.name}
          </span>
        ) : (
          <span className="badge badge-warning" title="Bu maçın oyuncusu değil">
            Yabancı
          </span>
        )}
        {wrongTeam && <span className="badge badge-danger">Yanlış takım</span>}
      </span>
      {slot.mods && <span className="lobby-slot-mods">{slot.mods}</span>}
      <span className={`lobby-slot-status ${score && !score.passed ? "is-failed" : ""}`}>{status}</span>
      {slot.team && (
        <IconButton
          icon="swap"
          label={`${slot.username} oyuncusunu ${slot.team === "red" ? "mavi" : "kırmızı"} takıma al`}
          disabled={!online || !inChannel}
          onClick={() => actions.moveToTeam(slot, slot.team === "red" ? "blue" : "red")}
        />
      )}
    </li>
  );
}
