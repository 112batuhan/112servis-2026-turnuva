import { useState } from "react";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { bestOfLabel, firstTo, type Race } from "../../../lib/bestOf";
import { formatClock } from "../../../lib/format";
import { roomLabel } from "../../../lib/liveRoom";
import { poolEntries, type MatchTeam } from "../../../lib/matchPool";
import type { RefereeRoomState } from "./useRefereeRoom";
import { ScoreSheet } from "../../../components/match/ScoreSheet";

/**
 * The match's score sheet. In a bracket match the two teams are columns that run from the score
 * at the top down through every map: each map's totals sit in them, the winner's cell lit, so the
 * figure at the top is the count of lit cells under it. Maps are newest first, the one that just
 * ended open to its players, ranked by score, each score in their team's column; the others open on
 * demand. "Sayma" leaves a map out (its cells go unlit). Maps the referee awarded by hand follow,
 * then where the results are read from and the tools for awarding a map or a forfeit. A lobby has
 * no score: its sheet lists each map's leading team. Maps outside the pool are left out.
 */
export function MatchHistory({ referee }: { referee: RefereeRoomState }) {
  const { room, finished, roster, ignored, actions, canRun, busy, tally, teams, race } = referee;
  if (!room) return null;
  const concluded = room.concluded_at !== null;
  const editable = teams.length === 2 && tally !== null && canRun && !concluded;
  const runs = canRun && !concluded && room.rooms.length > 0;
  return (
    <ScoreSheet
      room={room}
      teams={teams}
      tally={tally}
      finished={finished}
      roster={roster}
      status={<ScoreStatus referee={referee} />}
      raceAction={(r) => <RaceAction referee={referee} race={r} />}
      ignore={{ ignored, disabled: concluded, toggle: actions.toggleIgnored }}
      renderManualAction={
        editable
          ? (manual, label) => (
              <IconButton
                icon="undo"
                className="sheet-toggle"
                label={`${label}: hakem kararını geri al`}
                disabled={busy}
                onClick={() => actions.removeManualResult(manual.id)}
              />
            )
          : undefined
      }
      footer={
        <>
          {room.rooms.length > 0 && <ResultSource referee={referee} />}
          {editable && (
            <div className="sheet-tools">
              <ManualResultForm referee={referee} teams={teams} />
              <ForfeitControl referee={referee} teams={teams} />
            </div>
          )}
          {/* A decided race offers it in its line under the score instead. */}
          {runs && race?.kind !== "won" && <ConcludeControl referee={referee} />}
        </>
      }
    />
  );
}

/** The referee's control in the race line: concluding a match a team has won. */
function RaceAction({ referee, race }: { referee: RefereeRoomState; race: Race }) {
  const { room, canRun } = referee;
  if (!room || !canRun || room.concluded_at !== null || race.kind !== "won") return null;
  return <ConcludeControl referee={referee} />;
}

/**
 * Whether the site has the score as the bots count it. They save it as each map finishes, but not
 * over a score a host entered by hand, which stays until a map changes it or the referee saves.
 */
function ScoreStatus({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, live, scoreSaved, canRun, busy, actions } = referee;
  if (!room) return null;
  if (room.forfeit_slot) return <>Hükmen: {teams[room.forfeit_slot - 1]?.name} çekildi</>;
  if (room.concluded_at !== null) return <>Maç bitti</>;
  if (!live?.tally) return null;
  if (scoreSaved)
    return <span title="Odanın botu map bittikçe skoru sitede günceller">Sitede güncel</span>;
  if (canRun) {
    return (
      <>
        {room.team1_score === null && room.team2_score === null
          ? "Sitede henüz skor yok"
          : `Sitede ${room.team1_score ?? "–"} – ${room.team2_score ?? "–"}`}
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={busy}
          onClick={actions.saveScore}
        >
          Skoru kaydet
        </button>
      </>
    );
  }
  return null;
}

/**
 * Where the results are read from: the rooms' chat, as the bot reads the maps finish. When it
 * missed something (the bot dropped, a line was misread), the referee syncs them from osu!'s record
 * of the match's rooms (the mp link), which then takes the place of the chat's maps up to that
 * point for everyone; maps played after it are read from the chat again. It's only read when the
 * referee asks. A concluded match shows what was saved with its score.
 */
function ResultSource({ referee }: { referee: RefereeRoomState }) {
  const { room, source, osuSyncedAt, syncing, busy, canRun, actions } = referee;
  if (source === "saved") return <span className="sheet-source">Skorla kaydedilen sonuçlar</span>;
  // Shared by everyone on the page, so only those who run the match change it.
  const locked = !canRun || room?.concluded_at != null;
  const at = osuSyncedAt ? formatClock(osuSyncedAt) : null;
  return (
    <span className="sheet-source">
      {at
        ? `Oda sohbetinden okunuyor, ${at}'e kadarki map'ler maç linkinden`
        : "Oda sohbetinden okunuyor"}
      <button
        type="button"
        className="btn btn-secondary btn-sm"
        disabled={syncing || busy || locked}
        title="Sonuçları osu!'nun maç kaydından (mp linki) alıp sohbetten okunanların yerine koyar ve skoru kaydeder. Bot bir map'i kaçırdıysa ya da yanlış okuduysa düzeltir."
        onClick={actions.syncFromOsu}
      >
        <Icon name="refresh" size={14} />
        {syncing ? "Okunuyor…" : "Maç linkinden eşitle"}
      </button>
    </span>
  );
}

/**
 * Concludes the match. A bracket match needs a winner: it's concluded with the score as the bots
 * count it (saved first if the site doesn't have it yet), and the teams move on to their next
 * matches. Behind a confirmation, since reopening it takes a host.
 */
function ConcludeControl({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, tally, busy, actions, race, bestOf } = referee;
  if (!room) return null;
  const label = roomLabel(room);
  let blocker: string | null = null;
  let detail = "";
  if (room.kind === "bracket") {
    if (teams.length !== 2 || !tally) blocker = "İki takım da belli olmalı";
    else if (tally.team1 === tally.team2) blocker = "Skor berabere: önce kazanan belli olmalı";
    else {
      const winner = tally.team1 > tally.team2 ? teams[0] : teams[1];
      detail = `\nSkor: ${tally.team1} – ${tally.team2} (${winner.name} kazanır)`;
      // Allowed, since something may have happened, but said.
      if (bestOf !== null && race && race.kind !== "won") {
        detail += `\n\nDikkat: ${bestOfLabel(bestOf)} maçta kazanmak için ${firstTo(bestOf)} map gerekiyor.`;
      }
    }
  }
  return (
    <div className="sheet-conclude">
      <button
        type="button"
        className="btn btn-primary btn-sm"
        disabled={busy || blocker !== null}
        title={blocker ?? undefined}
        onClick={() => {
          if (
            window.confirm(
              `${label} bitti olarak işaretlensin mi?${detail}\n\nGeri almak için bir host maçı yeniden açmalı.`,
            )
          ) {
            actions.conclude();
          }
        }}
      >
        <Icon name="check" size={14} />
        Maçı bitir
      </button>
      {blocker && <span className="sheet-hint">{blocker}</span>}
    </div>
  );
}

/**
 * Awards a map to a team by hand, for one the room didn't settle (a disconnect, a result
 * Bancho never reported), or a point without a map. It counts with the room's results.
 */
function ManualResultForm({ referee, teams }: { referee: RefereeRoomState; teams: MatchTeam[] }) {
  const { room, busy, actions } = referee;
  const [open, setOpen] = useState(false);
  const [mapId, setMapId] = useState("");
  const [note, setNote] = useState("");
  if (!room) return null;
  const banned = new Set(room.bans.map((b) => b.map_id));
  const entries = poolEntries(room).filter((e) => !banned.has(e.map.id));

  if (!open) {
    return (
      <button type="button" className="btn btn-secondary btn-sm" onClick={() => setOpen(true)}>
        <Icon name="plus" size={14} />
        Hakem kararıyla map ver
      </button>
    );
  }

  const award = (teamId: string) => {
    actions.addManualResult(teamId, mapId || null, note.trim());
    setMapId("");
    setNote("");
    setOpen(false);
  };

  return (
    <div className="sheet-manual" role="group" aria-labelledby="sheet-manual-title">
      <div className="sheet-manual-head">
        <h4 id="sheet-manual-title" className="sheet-manual-title">
          Hakem kararıyla map ver
        </h4>
        <IconButton icon="close" label="Vazgeç" onClick={() => setOpen(false)} />
      </div>
      <p className="sheet-hint">
        Oda sonucu vermediyse (kopma, Bancho'nun bildirmediği bir skor) ya da ceza puanı için. Skora
        bir map olarak eklenir.
      </p>
      <div className="sheet-manual-fields">
        <label className="sheet-manual-field">
          <span>Map</span>
          <select
            className="select select-sm"
            value={mapId}
            onChange={(e) => setMapId(e.target.value)}
          >
            <option value="">Map yok (sadece puan)</option>
            {entries.map((e) => (
              <option key={e.map.id} value={e.map.id}>
                {e.label} · {e.map.artist} - {e.map.title}
              </option>
            ))}
          </select>
        </label>
        <label className="sheet-manual-field">
          <span>Not</span>
          <input
            className="input input-sm"
            value={note}
            maxLength={200}
            placeholder="Neden? (isteğe bağlı)"
            onChange={(e) => setNote(e.target.value)}
          />
        </label>
      </div>
      <div className="sheet-manual-teams">
        {teams.map((team) => (
          <button
            key={team.id}
            type="button"
            className={`btn btn-secondary btn-sm sheet-manual-team is-team-${team.color}`}
            disabled={busy}
            onClick={() => award(team.id)}
          >
            {team.name} alsın
          </button>
        ))}
      </div>
    </div>
  );
}

/**
 * Concludes the match by forfeit when a team didn't show up or the match couldn't be finished:
 * the other team wins and moves on. Behind a disclosure and a confirmation, since it can't be
 * undone without a host.
 */
function ForfeitControl({ referee, teams }: { referee: RefereeRoomState; teams: MatchTeam[] }) {
  const { busy, actions } = referee;
  return (
    <details className="sheet-forfeit">
      <summary>Hükmen bitir…</summary>
      <p className="sheet-hint">
        Bir takım maça gelmediyse ya da maç bir sebeple tamamlanamadıysa çekilen takımı seç. Diğer
        takım kazanır ve bir sonraki maça geçer; skor yerine W / FF yazar. Geri almak için bir host
        maçı yeniden açmalı.
      </p>
      <div className="sheet-manual-teams">
        {teams.map((team, i) => {
          const other = teams[1 - i];
          return (
            <button
              key={team.id}
              type="button"
              className="btn btn-danger btn-sm"
              disabled={busy}
              onClick={() => {
                if (
                  window.confirm(
                    `${team.name} çekilmiş sayılsın mı? Maç hükmen biter ve ${other.name} kazanır.`,
                  )
                ) {
                  actions.forfeit((i + 1) as 1 | 2);
                }
              }}
            >
              {team.name} çekildi
            </button>
          );
        })}
      </div>
    </details>
  );
}
