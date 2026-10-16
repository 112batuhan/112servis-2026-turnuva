import { useState, type ReactNode } from "react";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { LiveMark } from "../../../components/LiveMark";
import { splitMods, type MapResult } from "../../../lib/bancho";
import { entryOf, poolEntries, type MatchTeam, type PoolEntry } from "./pool";
import {
  scorer,
  teamTotals,
  type RefereeRoomState,
  type ResultRow,
  type RosterPlayer,
  type TeamTotal,
} from "./useRefereeRoom";
import "./MatchHistory.css";

interface PlayerLine {
  key: string;
  name: string;
  /** "HDHR", "NM", or "?" when they weren't read. */
  mods: string;
  score: number;
  passed: boolean;
  teamId: string | undefined;
}

/**
 * Each player's score on a finished map with the mods they played it with, best first. NoFail is
 * left out unless the players picked their own mods, since the referee adds it to every other
 * map. "?" is a player whose mods weren't read (the chat only knows them from !mp settings).
 */
function playerLines(result: MapResult, freemod: boolean, roster: RosterPlayer[]): PlayerLine[] {
  return result.scores
    .map((score, i) => {
      const r = scorer(score, roster);
      const mods =
        score.mods === null
          ? "?"
          : splitMods(score.mods)
              .filter((m) => freemod || m !== "NF")
              .join("") || "NM";
      return {
        key: `${i}`,
        name: r?.player.username ?? (score.username || `#${score.userId}`),
        mods,
        score: score.score,
        passed: score.passed,
        teamId: r?.team.id,
      };
    })
    .sort((a, b) => b.score - a.score);
}

const formatScore = (score: number) => score.toLocaleString("tr-TR");

/**
 * The match's score sheet. In a bracket match the two teams are columns that run from the score
 * at the top down through every map: each map's totals sit in them, the winner's cell lit, so the
 * figure at the top is the count of lit cells under it. Maps are newest first, the one that just
 * ended open to its players, each player's score in their team's column; the others open on
 * demand. "Sayma" leaves a map out (its cells go unlit). Maps the referee awarded by hand follow,
 * then where the results are read from and the tools for awarding a map or a forfeit. A lobby has
 * no score: its sheet lists each map's leading team. Maps outside the pool are left out.
 */
export function MatchHistory({ referee }: { referee: RefereeRoomState }) {
  const { room, finished, roster, ignored, actions, canRun, busy, tally, teams } = referee;
  const [open, setOpen] = useState<Record<string, boolean>>({});
  if (!room) return null;
  const bracket = teams.length === 2 && tally !== null;
  const entries = poolEntries(room);
  const concluded = room.concluded_at !== null;
  const editable = bracket && canRun && !concluded;

  // A lobby has no score: just its finished pool maps.
  const rows: ResultRow[] =
    tally?.rows ??
    finished.flatMap(({ result, key }) => {
      const entry = entryOf(entries, result, new Set());
      return entry ? [{ result, key, entry, totals: teamTotals(result, roster), winnerId: null }] : [];
    });
  const outsidePool = tally?.outsidePool ?? finished.length - rows.length;
  const latest = rows.length > 0 ? rows[rows.length - 1].key : null;
  const isOpen = (key: string) => open[key] ?? key === latest;
  const teamName = (id: string) => room.teams.find((t) => t.id === id)?.name ?? "?";
  const empty = rows.length === 0 && room.manual_results.length === 0;

  return (
    <section className={`sheet ${bracket ? "is-versus" : "is-lobby"}`} aria-labelledby="sheet-title">
      <div className="sheet-head">
        <div className="sheet-head-main">
          <h3 id="sheet-title" className="sheet-title">
            {bracket ? "Skor" : "Maplar"}
          </h3>
          <p className="sheet-status">
            {bracket ? <ScoreStatus referee={referee} /> : `${rows.length} map oynandı`}
          </p>
        </div>
        {bracket && <ScoreHeads referee={referee} teams={teams} />}
        <div className="sheet-head-end" />
      </div>

      {empty && (
        <div className="sheet-row is-empty">
          <span />
          <span className="sheet-empty">
            Havuzdan bir map bitince burada skorlarıyla listelenir{bracket ? "; kazanan takımın hücresi yanar" : ""}.
          </span>
          {bracket && teams.map((team) => <span key={team.id} className={`sheet-cell is-team-${team.color}`} />)}
        </div>
      )}

      {rows.length > 0 && (
        <ol className="sheet-list" reversed>
          {rows
            .map((row, i) => ({ row, n: i + 1 }))
            .reverse()
            .map(({ row: { result, key, entry, totals }, n }) => {
              const pick = room.picks.find((p) => p.map_id === entry.map.id);
              const pickedBy = pick && teams.find((t) => t.id === pick.team_id);
              const off = ignored.has(key);
              const freemod = entry.freemod || result.freemod;
              const players = playerLines(result, freemod, roster);
              const expanded = isOpen(key);
              return (
                <li
                  key={key}
                  className={`sheet-map${off ? " is-ignored" : ""}${expanded ? " is-open" : ""}${freemod ? " is-freemod" : ""}`}
                >
                  <div className="sheet-row">
                    <span className="sheet-order">{n}</span>
                    <MapCell entry={entry} name={result.beatmapName}>
                      {pickedBy && <span className={`sheet-pick is-team-${pickedBy.color}`}>{pickedBy.name} seçti</span>}
                      {off && <span className="sheet-sub-note">sayılmıyor</span>}
                    </MapCell>
                    {bracket ? (
                      <TotalCells teams={teams} totals={totals} counted={!off} />
                    ) : (
                      <span className="sheet-leader">
                        {totals[0] ? (
                          <>
                            <span className="sheet-leader-name">{teamName(totals[0].teamId)}</span>
                            <span className="sheet-leader-score">{formatScore(totals[0].score)}</span>
                          </>
                        ) : (
                          <span className="sheet-sub-note">Skor yok</span>
                        )}
                      </span>
                    )}
                    <label className="sheet-ignore" title="Isınma ya da tekrar gibi sayılmaması gereken bir map">
                      <input
                        type="checkbox"
                        checked={off}
                        disabled={concluded}
                        onChange={() => actions.toggleIgnored(key)}
                      />
                      <span className="sheet-ignore-text">Sayma</span>
                    </label>
                    <IconButton
                      icon="chevronDown"
                      className="sheet-toggle"
                      label={`${entry.label}: ${expanded ? "oyuncu skorlarını gizle" : "oyuncu skorlarını göster"}`}
                      aria-expanded={expanded}
                      onClick={() => setOpen((o) => ({ ...o, [key]: !expanded }))}
                    />
                  </div>
                  {expanded &&
                    (bracket ? (
                      <PlayerRows teams={teams} players={players} />
                    ) : (
                      <LobbyTotals teamName={teamName} totals={totals} players={players} />
                    ))}
                </li>
              );
            })}
        </ol>
      )}

      {room.manual_results.length > 0 && (
        <ul className="sheet-list is-manual" aria-label="Hakem kararları">
          {room.manual_results.map((manual) => {
            const entry = entries.find((e) => e.map.id === manual.map_id);
            const note = <span className="sheet-sub-note">Hakem kararı{manual.note && `: ${manual.note}`}</span>;
            return (
              <li key={manual.id} className="sheet-map">
                <div className="sheet-row">
                  <span className="sheet-order" aria-hidden="true">
                    <Icon name="gavel" size={15} />
                  </span>
                  {entry ? (
                    <MapCell entry={entry} name={null}>
                      {note}
                    </MapCell>
                  ) : (
                    <span className="sheet-map-cell">
                      <span className="sheet-label is-blank" aria-hidden="true" />
                      <span className="sheet-map-text">
                        <span className="sheet-map-title">Map dışı puan</span>
                        <span className="sheet-sub">{note}</span>
                      </span>
                    </span>
                  )}
                  {teams.map((team) => (
                    <span
                      key={team.id}
                      className={`sheet-cell is-team-${team.color}${team.id === manual.team_id ? " is-winner" : ""}`}
                    >
                      {team.id === manual.team_id ? "+1" : "–"}
                    </span>
                  ))}
                  <span />
                  {editable ? (
                    <IconButton
                      icon="undo"
                      className="sheet-toggle"
                      label={`${entry?.label ?? "Puan"}: hakem kararını geri al`}
                      disabled={busy}
                      onClick={() => actions.removeManualResult(manual.id)}
                    />
                  ) : (
                    <span />
                  )}
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <footer className="sheet-foot">
        {room.rooms.length > 0 && <ResultSource referee={referee} />}
        {outsidePool > 0 && (
          <span className="sheet-note">Havuzda olmayan {outsidePool} map (ısınma ya da yanlış map) sayılmıyor.</span>
        )}
        {editable && (
          <div className="sheet-tools">
            <ManualResultForm referee={referee} teams={teams} />
            <ForfeitControl referee={referee} teams={teams} />
          </div>
        )}
      </footer>
    </section>
  );
}

/**
 * The tops of the two team columns: each team's name over its figure, the side ahead in full
 * ink. A concluded match shows its saved result, a forfeit W and FF.
 */
function ScoreHeads({ referee, teams }: { referee: RefereeRoomState; teams: MatchTeam[] }) {
  const { room, tally } = referee;
  if (!room || !tally) return null;
  const forfeit = room.forfeit_slot;
  const figures: [string, string] = forfeit
    ? forfeit === 1
      ? ["FF", "W"]
      : ["W", "FF"]
    : room.concluded_at !== null
      ? [String(room.team1_score ?? "–"), String(room.team2_score ?? "–")]
      : [String(tally.team1), String(tally.team2)];
  const lead = figures[0] === figures[1] ? null : Number(figures[0]) > Number(figures[1]) || figures[0] === "W" ? 0 : 1;
  return (
    <>
      {teams.map((team, i) => (
        <div key={team.id} className={`sheet-team is-team-${team.color}${lead === i ? " is-ahead" : ""}`}>
          <span className="sheet-team-name" title={team.name}>
            {team.name}
          </span>
          <span className="sheet-team-figure" aria-label={`${team.name}: ${figures[i]}`}>
            {figures[i]}
          </span>
        </div>
      ))}
    </>
  );
}

/** Whether the site shows the score as it is here, and the way to save it when it doesn't. */
function ScoreStatus({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, liveScore, scoreSaving, scoreSaved, canRun, busy, actions } = referee;
  if (!room) return null;
  if (room.forfeit_slot) return <>Hükmen: {teams[room.forfeit_slot - 1]?.name} çekildi</>;
  if (room.concluded_at !== null) return <>Maç bitti</>;
  if (scoreSaving) return <>Kaydediliyor…</>;
  if (liveScore && scoreSaved) {
    return (
      <span className="sheet-live" title="Map bittikçe skor sitede güncellenir">
        <LiveMark compact /> sitede
      </span>
    );
  }
  if (canRun && !scoreSaved) {
    return (
      <>
        {room.team1_score === null && room.team2_score === null
          ? "Sitede henüz skor yok"
          : `Sitede ${room.team1_score ?? "–"} – ${room.team2_score ?? "–"}`}
        <button type="button" className="btn btn-secondary btn-sm" disabled={busy} onClick={actions.saveScore}>
          Skoru kaydet
        </button>
      </>
    );
  }
  return null;
}

function MapCell({ entry, name, children }: { entry: PoolEntry; name: string | null; children?: ReactNode }) {
  const { map } = entry;
  const full = name ?? `${map.artist} - ${map.title} [${map.version}]`;
  return (
    <span className="sheet-map-cell">
      <span className="sheet-label" style={{ color: entry.color, borderColor: entry.color }}>
        {entry.label}
      </span>
      <span className="sheet-map-text">
        <span className="sheet-map-title" title={full}>
          {name ?? (
            <>
              {map.artist} - {map.title} <span className="sheet-map-version">[{map.version}]</span>
            </>
          )}
        </span>
        <span className="sheet-sub">{children}</span>
      </span>
    </span>
  );
}

/** A map's two totals in the team columns; the higher one is lit when the map counts. */
function TotalCells({ teams, totals, counted }: { teams: MatchTeam[]; totals: TeamTotal[]; counted: boolean }) {
  const scores = teams.map((t) => totals.find((x) => x.teamId === t.id)?.score ?? null);
  const [a, b] = scores;
  const winner = !counted || a === null || b === null || a === b ? null : a > b ? 0 : 1;
  return (
    <>
      {teams.map((team, i) => (
        <span
          key={team.id}
          className={`sheet-cell is-team-${team.color}${winner === i ? " is-winner" : ""}`}
          aria-label={`${team.name}: ${scores[i] === null ? "skor yok" : formatScore(scores[i])}${winner === i ? ", kazandı" : ""}`}
        >
          {scores[i] === null ? "–" : formatScore(scores[i])}
        </span>
      ))}
    </>
  );
}

/**
 * An opened bracket map: one line per player, red's then blue's, the name and mods under the
 * map's name and the score in the player's team column, so it adds up to the total above it.
 */
function PlayerRows({ teams, players }: { teams: MatchTeam[]; players: PlayerLine[] }) {
  const others = players.filter((p) => !teams.some((t) => t.id === p.teamId));
  return (
    <ul className="sheet-players" aria-label="Oyuncu skorları">
      {teams.flatMap((team) =>
        players
          .filter((p) => p.teamId === team.id)
          .map((p) => (
            <li key={p.key} className="sheet-row is-player">
              <span />
              <span className="sheet-player">
                <span className={`sheet-player-dot is-team-${team.color}`} aria-hidden="true" />
                <span className="sheet-player-name">{p.name}</span>
                <span className={`sheet-mods${p.mods === "?" || p.mods === "NM" ? " is-plain" : ""}`}>{p.mods}</span>
              </span>
              {teams.map((column) => (
                <span key={column.id} className={`sheet-cell is-team-${column.color} is-player-score`}>
                  {column.id === team.id && (
                    <span className={p.passed ? undefined : "is-failed"}>
                      {formatScore(p.score)}
                      {!p.passed && <span className="sr-only"> (geçemedi)</span>}
                    </span>
                  )}
                </span>
              ))}
              <span />
              <span />
            </li>
          )),
      )}
      {others.length > 0 && (
        <li className="sheet-row is-player is-others">
          <span />
          <span className="sheet-sub-note">
            Maçın oyuncusu değil (sayılmadı): {others.map((p) => `${p.name} ${formatScore(p.score)}`).join(", ")}
          </span>
        </li>
      )}
    </ul>
  );
}

/** A lobby's map, opened: every team best first, with its players under it. */
function LobbyTotals({
  teamName,
  totals,
  players,
}: {
  teamName: (id: string) => string;
  totals: TeamTotal[];
  players: PlayerLine[];
}) {
  if (totals.length === 0) return <p className="sheet-lobby-empty">Skor yok</p>;
  return (
    <ol className="sheet-lobby">
      {totals.map((t, i) => (
        <li key={t.teamId}>
          <span className="sheet-lobby-head">
            <span className="sheet-lobby-place">{i + 1}</span>
            <span className="sheet-lobby-name">{teamName(t.teamId)}</span>
            <span className="sheet-lobby-score">{formatScore(t.score)}</span>
          </span>
          {players
            .filter((p) => p.teamId === t.teamId)
            .map((p) => (
              <span key={p.key} className="sheet-lobby-player">
                <span className="sheet-player-name">{p.name}</span>
                <span className={`sheet-mods${p.mods === "?" || p.mods === "NM" ? " is-plain" : ""}`}>{p.mods}</span>
                <span className={`sheet-lobby-player-score${p.passed ? "" : " is-failed"}`}>{formatScore(p.score)}</span>
              </span>
            ))}
        </li>
      ))}
    </ol>
  );
}

/**
 * Where the results are read from. The rooms' chat by default; when it missed something (the
 * connection dropped, nobody had the page open), the referee syncs them from osu!'s record of
 * the match's rooms, which they're then counted from, and saved, until the referee goes back.
 * A concluded match shows what was saved with its score, with nothing to change.
 */
function ResultSource({ referee }: { referee: RefereeRoomState }) {
  const { source, osuSyncedAt, syncing, busy, actions } = referee;
  if (source === "saved") return <span className="sheet-source">Skorla kaydedilen sonuçlar</span>;
  const at = osuSyncedAt
    ? new Date(osuSyncedAt).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" })
    : null;
  return (
    <span className="sheet-source">
      {source === "osu" ? `Maç linkinden okundu (${at})` : "Oda sohbetinden okunuyor"}
      <button
        type="button"
        className="btn btn-secondary btn-sm"
        disabled={syncing || busy}
        title="Sonuçları osu!'nun maç kaydından (mp linki) al. Sohbet bir map'i kaçırdıysa düzeltir."
        onClick={actions.syncFromOsu}
      >
        <Icon name="refresh" size={14} />
        {syncing ? "Okunuyor…" : source === "osu" ? "Yeniden oku" : "Maç linkinden eşitle"}
      </button>
      {source === "osu" && (
        <button type="button" className="btn btn-secondary btn-sm" disabled={syncing || busy} onClick={actions.readFromChat}>
          Sohbete dön
        </button>
      )}
    </span>
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
        Oda sonucu vermediyse (kopma, Bancho'nun bildirmediği bir skor) ya da ceza puanı için. Skora bir map olarak
        eklenir.
      </p>
      <div className="sheet-manual-fields">
        <label className="sheet-manual-field">
          <span>Map</span>
          <select className="select select-sm" value={mapId} onChange={(e) => setMapId(e.target.value)}>
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
        Bir takım maça gelmediyse ya da maç bir sebeple tamamlanamadıysa çekilen takımı seç. Diğer takım kazanır ve
        bir sonraki maça geçer; skor yerine W / FF yazar. Geri almak için bir host maçı yeniden açmalı.
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
                if (window.confirm(`${team.name} çekilmiş sayılsın mı? Maç hükmen biter ve ${other.name} kazanır.`)) {
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
