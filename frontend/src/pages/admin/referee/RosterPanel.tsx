import { useEffect, useState, type ReactNode } from "react";
import { errorMessage } from "../../../api/client";
import { pingPlayer } from "../../../api/discord";
import type { AdminTeamPlayer, Team } from "../../../api/types";
import { DiscordMention } from "../../../components/DiscordMention";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { LiveMark } from "../../../components/LiveMark";
import { sameName, splitMods, type MapResult, type SlotTeam } from "../../../lib/bancho";
import { formatRank } from "../../../lib/format";
import { entryOf, matchTeams, poolEntries, type MatchTeam } from "./pool";
import { scorer, teamTotals, type RefereeRoomState, type RosterPlayer } from "./useRefereeRoom";
import "./RosterPanel.css";

/** The match's teams and players with their Discords and where they are in the room. */
export function RosterPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, roster } = referee;
  if (!room) return null;
  const colorOf = (team: Team<AdminTeamPlayer>) => roster.find((r) => r.team.id === team.id)?.color ?? null;

  return (
    <section className="roster-panel" aria-label="Takımlar">
      <h2 className="roster-title">Takımlar</h2>
      {room.teams.length === 0 && <p className="muted">Bu maçta henüz takım yok.</p>}
      {room.teams.map((team) => (
        <TeamBlock key={team.id} team={team} color={colorOf(team)} referee={referee} />
      ))}
      <Results referee={referee} />
    </section>
  );
}

function TeamBlock({
  team,
  color,
  referee,
}: {
  team: Team<AdminTeamPlayer>;
  color: SlotTeam | null;
  referee: RefereeRoomState;
}) {
  const { lobby, actions, inChannel, room, canRun } = referee;
  return (
    <div className={`roster-team ${color ? `is-team-${color}` : ""}`}>
      <h3 className="roster-team-name">
        {team.name}
        {color && <span className="roster-team-color">{color === "red" ? "Kırmızı" : "Mavi"}</span>}
      </h3>
      <ul className="roster-players">
        {team.players.map((p) => {
          const slot = lobby.slots.find((s) => s.userId === p.osu_id || sameName(s.username, p.username));
          return (
            <li key={p.id} className="roster-player">
              <img className="roster-avatar" src={p.avatar_url} alt="" loading="lazy" />
              <span className="roster-player-text">
                <a href={`https://osu.ppy.sh/users/${p.osu_id}`} target="_blank" rel="noopener noreferrer">
                  {p.username}
                </a>
                <span className="roster-player-meta">
                  {formatRank(p.global_rank)} ·{" "}
                  {room?.discord_channel && canRun ? (
                    <DiscordPing matchId={room.id} player={p} />
                  ) : (
                    <DiscordMention player={p} fallback="Discord yok" />
                  )}
                </span>
              </span>
              {slot ? (
                <span className="badge badge-success" title="Odadaki slotu">
                  Slot {slot.slot}
                </span>
              ) : (
                <span className="badge">Odada değil</span>
              )}
              <IconButton
                icon="userPlus"
                label={`${p.username} oyuncusunu odaya davet et`}
                disabled={!inChannel}
                onClick={() => actions.invite(p.osu_id)}
              />
            </li>
          );
        })}
      </ul>
    </div>
  );
}

const PING_NOTE_MS = 2500;

/**
 * A player's Discord name that has the bot ping them in the match's Discord channel when
 * clicked, to call them into the room.
 */
function DiscordPing({ matchId, player }: { matchId: string; player: AdminTeamPlayer }) {
  const [state, setState] = useState<{ kind: "idle" | "busy" | "done" } | { kind: "failed"; error: string }>({
    kind: "idle",
  });

  useEffect(() => {
    if (state.kind !== "done" && state.kind !== "failed") return;
    const timer = setTimeout(() => setState({ kind: "idle" }), PING_NOTE_MS);
    return () => clearTimeout(timer);
  }, [state]);

  if (!player.discord_username) return <>Discord yok</>;
  if (player.discord_in_guild === false) {
    return (
      <span className="roster-discord-left" title="Turnuvanın Discord sunucusundan ayrıldı">
        @{player.discord_username} ⚠<span className="sr-only"> (sunucuda değil)</span>
      </span>
    );
  }

  async function ping() {
    setState({ kind: "busy" });
    try {
      await pingPlayer(matchId, player.id);
      setState({ kind: "done" });
    } catch (e) {
      setState({ kind: "failed", error: errorMessage(e) });
    }
  }

  return (
    <button
      type="button"
      className="roster-discord-ping"
      title={state.kind === "failed" ? state.error : "Maçın Discord kanalında etiketle"}
      aria-label={`@${player.discord_username}: maçın Discord kanalında etiketle`}
      disabled={state.kind === "busy"}
      onClick={ping}
    >
      {state.kind === "done" ? "Etiketlendi" : state.kind === "failed" ? "Etiketlenemedi" : `@${player.discord_username}`}
      <span className="sr-only" role="status">
        {state.kind === "done" ? "Oyuncu etiketlendi" : state.kind === "failed" ? state.error : ""}
      </span>
    </button>
  );
}

/**
 * The match's results. A bracket match gets the score as a board, saved to the site as it
 * changes; each finished pool map with its label, who picked it and the team totals, the
 * winner in its team's colour; the maps the referee awarded by hand; and the forfeit. Maps
 * that aren't in the pool (warmups, mistakes) are left out entirely.
 */
function Results({ referee }: { referee: RefereeRoomState }) {
  const { room, finished, roster, ignored, actions, canRun, busy, tally } = referee;
  if (!room) return null;
  const teams = matchTeams(room);
  const bracket = teams.length === 2 && tally !== null;
  if (!bracket && finished.length === 0 && room.rooms.length === 0) return null;
  const name = (teamId: string) => room.teams.find((t) => t.id === teamId)?.name ?? "?";
  const teamClass = (teamId: string) => {
    const team = teams.find((t) => t.id === teamId);
    return team ? `is-team-${team.color}` : "";
  };
  const entries = poolEntries(room);
  const concluded = room.concluded_at !== null;
  const editable = bracket && canRun && !concluded;

  // A lobby has no score: just its finished pool maps.
  const rows =
    tally?.rows ??
    finished.flatMap(({ result, key }) => {
      const entry = entryOf(entries, result, new Set());
      return entry ? [{ result, key, entry, totals: teamTotals(result, roster), winnerId: null }] : [];
    });
  const outsidePool = tally?.outsidePool ?? finished.length - rows.length;

  return (
    <div className="roster-results">
      <h3 className="roster-title">Sonuçlar</h3>
      {bracket && <ScoreBoard referee={referee} teams={teams} />}
      {room.rooms.length > 0 && <ResultSourceControl referee={referee} />}
      {rows.length === 0 && room.manual_results.length === 0 && (
        <p className="muted">Havuzdan henüz map oynanmadı.</p>
      )}
      {(rows.length > 0 || room.manual_results.length > 0) && (
        <ol className="roster-result-list">
          {rows.map(({ result, key, entry, totals }) => {
            const tied = totals.length > 1 && totals[0].score === totals[1].score;
            const pick = room.picks.find((p) => p.map_id === entry.map.id);
            return (
              <li key={key} className={ignored.has(key) ? "is-ignored" : undefined}>
                <span className="roster-result-map">
                  <span className="roster-result-label" style={{ borderColor: entry.color, color: entry.color }}>
                    {entry.label}
                  </span>
                  <span className="roster-result-name">
                    {result.beatmapName ?? `${entry.map.artist} - ${entry.map.title}`}
                  </span>
                </span>
                <span className="roster-result-totals">
                  {totals.length === 0 && <span className="muted">Skor yok</span>}
                  {totals.map((t, j) => (
                    <span
                      key={t.teamId}
                      className={j === 0 && totals.length > 1 && !tied ? `is-winner ${teamClass(t.teamId)}` : undefined}
                    >
                      {name(t.teamId)} {t.score.toLocaleString("tr-TR")}
                    </span>
                  ))}
                  {pick && (
                    <span className={`roster-result-pick ${teamClass(pick.team_id)}`}>{name(pick.team_id)} seçti</span>
                  )}
                </span>
                <PlayerScores
                  result={result}
                  freemod={entry.freemod || result.freemod}
                  roster={roster}
                  teamClass={teamClass}
                />
                <label className="roster-result-ignore">
                  <input
                    type="checkbox"
                    checked={ignored.has(key)}
                    disabled={concluded}
                    onChange={() => actions.toggleIgnored(key)}
                  />{" "}
                  Sayma
                </label>
              </li>
            );
          })}
          {room.manual_results.map((manual) => {
            const entry = entries.find((e) => e.map.id === manual.map_id);
            return (
              <li key={manual.id} className="is-manual">
                <span className="roster-result-map">
                  <span
                    className="roster-result-label"
                    style={entry ? { borderColor: entry.color, color: entry.color } : undefined}
                  >
                    {entry?.label ?? "—"}
                  </span>
                  <span className="roster-result-name">
                    {entry ? `${entry.map.artist} - ${entry.map.title}` : "Map dışı puan"}
                  </span>
                </span>
                <span className="roster-result-totals">
                  <span className={`is-winner ${teamClass(manual.team_id)}`}>{name(manual.team_id)} aldı</span>
                  <span>Hakem kararı{manual.note && `: ${manual.note}`}</span>
                </span>
                {editable && (
                  <IconButton
                    icon="undo"
                    label={`${entry?.label ?? "Puan"}: hakem kararını geri al`}
                    className="roster-result-ignore"
                    disabled={busy}
                    onClick={() => actions.removeManualResult(manual.id)}
                  />
                )}
              </li>
            );
          })}
        </ol>
      )}
      {outsidePool > 0 && (
        <p className="roster-result-hidden muted">Havuzda olmayan {outsidePool} map gösterilmiyor ve sayılmıyor.</p>
      )}
      {editable && <ManualResultForm referee={referee} teams={teams} />}
      {editable && <ForfeitControl referee={referee} teams={teams} />}
    </div>
  );
}

/**
 * Each player's score on a finished map with the mods they played it with, by team, best first.
 * NoFail is left out unless the players picked their own mods, since the referee adds it to every
 * other map. "?" is a player whose mods weren't read (the chat only knows them from !mp settings).
 */
function PlayerScores({
  result,
  freemod,
  roster,
  teamClass,
}: {
  result: MapResult;
  freemod: boolean;
  roster: RosterPlayer[];
  teamClass: (teamId: string) => string;
}) {
  if (result.scores.length === 0) return null;
  const rows = result.scores
    .map((score) => {
      const r = scorer(score, roster);
      const mods =
        score.mods === null
          ? "?"
          : splitMods(score.mods)
              .filter((m) => freemod || m !== "NF")
              .join("") || "NM";
      return { score, mods, name: r?.player.username ?? (score.username || `#${score.userId}`), teamId: r?.team.id };
    })
    .sort((a, b) => (a.teamId ?? "").localeCompare(b.teamId ?? "") || b.score.score - a.score.score);
  return (
    <ul className={`roster-result-players${freemod ? " is-freemod" : ""}`} aria-label="Oyuncu skorları">
      {rows.map(({ score, mods, name, teamId }, i) => (
        <li key={i} className={teamId ? teamClass(teamId) : undefined}>
          <span className="roster-result-player">{name}</span>
          <span className={`roster-result-mods${mods === "?" || mods === "NM" ? " is-plain" : ""}`}>{mods}</span>
          <span className={score.passed ? undefined : "is-failed"}>{score.score.toLocaleString("tr-TR")}</span>
        </li>
      ))}
    </ul>
  );
}

/**
 * The score as the match's head-to-head: the red team's figure and the blue team's, read from
 * the room plus the hand-awarded maps, and whether the site shows it yet.
 */
function ScoreBoard({ referee, teams }: { referee: RefereeRoomState; teams: MatchTeam[] }) {
  const { room, tally, liveScore, scoreSaving, scoreSaved: saved, canRun, busy, actions } = referee;
  if (!room || !tally) return null;
  const concluded = room.concluded_at !== null;
  const forfeit = room.forfeit_slot;
  // Once it's concluded the saved result stands; before that, the running tally.
  const figures: [string, string] = forfeit
    ? forfeit === 1
      ? ["FF", "W"]
      : ["W", "FF"]
    : concluded
      ? [String(room.team1_score ?? "–"), String(room.team2_score ?? "–")]
      : [String(tally.team1), String(tally.team2)];
  const lead = figures[0] === figures[1] ? null : Number(figures[0]) > Number(figures[1]) || figures[0] === "W" ? 0 : 1;

  let status: ReactNode = null;
  if (forfeit) {
    status = <span className="roster-score-status">Hükmen: {teams[forfeit - 1].name} çekildi</span>;
  } else if (concluded) {
    status = <span className="roster-score-status">Maç bitti</span>;
  } else if (scoreSaving) {
    status = <span className="roster-score-status">Kaydediliyor…</span>;
  } else if (liveScore && saved) {
    status = (
      <span className="roster-score-status is-live" title="Map bittikçe skor sitede güncellenir">
        <LiveMark compact /> sitede
      </span>
    );
  } else if (canRun && !saved) {
    status = (
      <span className="roster-score-status">
        Sitede {room.team1_score ?? "–"} – {room.team2_score ?? "–"}
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={busy}
          onClick={actions.saveScore}
        >
          Skoru kaydet
        </button>
      </span>
    );
  }

  return (
    <div className="roster-score">
      <div className="roster-score-board" aria-label="Skor">
        {teams.map((team, i) => (
          <div key={team.id} className={`roster-score-side is-team-${team.color}${lead === i ? " is-ahead" : ""}`}>
            <span className="roster-score-name">{team.name}</span>
            <span className="roster-score-figure">{figures[i]}</span>
          </div>
        ))}
      </div>
      {status}
    </div>
  );
}

/**
 * Where the results are read from. The rooms' chat by default; when it missed something (the
 * connection dropped, nobody had the page open), the referee syncs them from osu!'s record of
 * the match's rooms, which they're then counted from, and saved, until the referee goes back.
 * A concluded match shows what was saved with its score, with nothing to change.
 */
function ResultSourceControl({ referee }: { referee: RefereeRoomState }) {
  const { source, osuSyncedAt, syncing, busy, actions } = referee;
  if (source === "saved") {
    return (
      <div className="roster-result-source">
        <span className="roster-result-source-label">Maç bitti: skorla kaydedilen sonuçlar</span>
      </div>
    );
  }
  const at = osuSyncedAt
    ? new Date(osuSyncedAt).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" })
    : null;
  return (
    <div className="roster-result-source">
      <span className="roster-result-source-label">
        {source === "osu" ? `Maç linkinden okundu (${at})` : "Oda sohbetinden okunuyor"}
      </span>
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
      <button type="button" className="btn btn-secondary btn-sm roster-manual-open" onClick={() => setOpen(true)}>
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
    <div className="roster-manual" role="group" aria-label="Hakem kararıyla map ver">
      <div className="roster-manual-head">
        <span className="roster-manual-title">Hakem kararıyla map ver</span>
        <IconButton icon="close" label="Vazgeç" onClick={() => setOpen(false)} />
      </div>
      <p className="roster-manual-hint">
        Oda sonucu vermediyse (kopma, Bancho'nun bildirmediği bir skor) ya da ceza puanı için. Skora bir map olarak
        eklenir.
      </p>
      <label className="roster-manual-field">
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
      <label className="roster-manual-field">
        <span>Not</span>
        <input
          className="input input-sm"
          value={note}
          maxLength={200}
          placeholder="Neden? (isteğe bağlı)"
          onChange={(e) => setNote(e.target.value)}
        />
      </label>
      <div className="roster-manual-teams">
        {teams.map((team) => (
          <button
            key={team.id}
            type="button"
            className={`btn btn-secondary btn-sm roster-manual-team is-team-${team.color}`}
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
    <details className="roster-forfeit">
      <summary>Hükmen bitir…</summary>
      <p className="roster-manual-hint">
        Bir takım maça gelmediyse ya da maç bir sebeple tamamlanamadıysa çekilen takımı seç. Diğer takım kazanır ve
        bir sonraki maça geçer; skor yerine W / FF yazar. Geri almak için bir host maçı yeniden açmalı.
      </p>
      <div className="roster-manual-teams">
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
