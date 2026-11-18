import { useState, type ReactNode } from "react";
import { Icon } from "../Icon";
import { IconButton } from "../IconButton";
import { splitMods, type MapResult } from "../../lib/bancho";
import { bestOfLabel, firstTo, standingOf, type Standing } from "../../lib/bestOf";
import { formatScore } from "../../lib/format";
import type { ManualResult } from "../../api/types";
import {
  poolRows,
  scorer,
  type FinishedMap,
  type ResultRow,
  type RosterPlayer,
  type Tally,
  type TeamTotal,
} from "../../lib/liveRoom";
import { poolEntries, type MatchData, type MatchTeam, type PoolEntry } from "../../lib/matchPool";
import "./ScoreSheet.css";

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
        name: r?.player.username ?? (score.username || `#${score.userId ?? "?"}`),
        mods,
        score: score.score,
        passed: score.passed,
        teamId: r?.team.id,
      };
    })
    .sort((a, b) => b.score - a.score);
}

const NONE: Set<string> = new Set();

interface ScoreSheetProps {
  room: MatchData;
  /** A bracket match's two teams; none for a lobby. */
  teams: MatchTeam[];
  /** A bracket match's map wins; null for a lobby. */
  tally: Tally | null;
  /** Every finished map of the match's rooms, for a lobby's sheet. */
  finished: FinishedMap[];
  roster: RosterPlayer[];
  /** Under the title: where the score stands. */
  status: ReactNode;
  /** The referee's "Sayma" boxes, which leave a map out of the score. */
  ignore?: { ignored: Set<string>; disabled: boolean; toggle: (key: string) => void };
  /** The button that takes back a hand-awarded result. */
  renderManualAction?: (manual: ManualResult, label: string) => ReactNode;
  /** The referee's control in the standing line (concluding a won match). */
  standingAction?: (standing: Standing) => ReactNode;
  /** Under the maps. */
  footer?: ReactNode;
}

/** The score sheet itself, shared by the referee page and the public match page. */
export function ScoreSheet({
  room,
  teams,
  tally,
  finished,
  roster,
  status,
  ignore,
  renderManualAction,
  standingAction,
  footer,
}: ScoreSheetProps) {
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const ignored = ignore?.ignored ?? NONE;
  const bracket = teams.length === 2 && tally !== null;
  const entries = poolEntries(room);
  // The stage's best of: the sheet counts towards it (a concluded match by its saved score).
  const bestOf = bracket && room.kind === "bracket" ? (room.stage?.best_of ?? null) : null;
  const concluded = room.concluded_at !== null;
  const score: [number, number] | null = tally
    ? concluded
      ? [room.team1_score ?? 0, room.team2_score ?? 0]
      : [tally.team1, tally.team2]
    : null;
  const standing =
    bestOf !== null && score && !room.forfeit_slot ? standingOf(bestOf, score[0], score[1]) : null;

  // A lobby has no score: just its finished pool maps.
  const rows: ResultRow[] = tally?.rows ?? poolRows(finished, entries);
  const outsidePool = tally?.outsidePool ?? finished.length - rows.length;
  const latest = rows.length > 0 ? rows[rows.length - 1].key : null;
  const isOpen = (key: string) => open[key] ?? key === latest;
  const teamName = (id: string) => room.teams.find((t) => t.id === id)?.name ?? "?";
  const empty = rows.length === 0 && room.manual_results.length === 0;

  return (
    <section
      className={`sheet ${bracket ? "is-versus" : "is-lobby"}`}
      aria-labelledby="sheet-title"
    >
      <div className="sheet-head">
        <div className="sheet-head-main">
          <h3 id="sheet-title" className="sheet-title">
            {bracket ? "Skor" : "Maplar"}
            {bestOf !== null && (
              <span className="sheet-bo" title={`${firstTo(bestOf)} map alan kazanır`}>
                {bestOfLabel(bestOf)}
                <span className="sr-only">, {firstTo(bestOf)} map alan kazanır</span>
              </span>
            )}
          </h3>
          <p className="sheet-status">{bracket ? status : `${rows.length} map oynandı`}</p>
        </div>
        {bracket && <ScoreHeads room={room} tally={tally} teams={teams} bestOf={bestOf} />}
        <div className="sheet-head-end" />
      </div>

      {standing && score && !concluded && (
        <StandingLine
          standing={standing}
          score={score}
          teams={teams}
          room={room}
          action={standingAction}
        />
      )}

      {empty && (
        <div className="sheet-row is-empty">
          <span />
          <span className="sheet-empty">
            Havuzdan bir map bitince burada skorlarıyla listelenir
            {bracket ? "; kazanan takımın hücresi yanar" : ""}.
          </span>
          {bracket &&
            teams.map((team) => (
              <span key={team.id} className={`sheet-cell is-team-${team.color}`} />
            ))}
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
                      {pickedBy && (
                        <span className={`sheet-pick is-team-${pickedBy.color}`}>
                          {pickedBy.name} seçti
                        </span>
                      )}
                      {off && <span className="sheet-sub-note">sayılmıyor</span>}
                    </MapCell>
                    {bracket ? (
                      <TotalCells teams={teams} totals={totals} counted={!off} />
                    ) : (
                      <span className="sheet-leader">
                        {totals[0] ? (
                          <>
                            <span className="sheet-leader-name">{teamName(totals[0].teamId)}</span>
                            <span className="sheet-leader-score">
                              {formatScore(totals[0].score)}
                            </span>
                          </>
                        ) : (
                          <span className="sheet-sub-note">Skor yok</span>
                        )}
                      </span>
                    )}
                    {ignore ? (
                      <label
                        className="sheet-ignore"
                        title="Isınma ya da tekrar gibi sayılmaması gereken bir map"
                      >
                        <input
                          type="checkbox"
                          checked={off}
                          disabled={ignore.disabled}
                          onChange={() => ignore.toggle(key)}
                        />
                        <span className="sheet-ignore-text">Sayma</span>
                      </label>
                    ) : (
                      <span />
                    )}
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
            const note = (
              <span className="sheet-sub-note">
                Hakem kararı{manual.note && `: ${manual.note}`}
              </span>
            );
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
                  {renderManualAction?.(manual, entry?.label ?? "Puan") ?? <span />}
                </div>
              </li>
            );
          })}
        </ul>
      )}

      <footer className="sheet-foot">
        {footer}
        {outsidePool > 0 && (
          <span className="sheet-note">
            Havuzda olmayan {outsidePool} map (ısınma ya da yanlış map) sayılmıyor.
          </span>
        )}
      </footer>
    </section>
  );
}

/**
 * The tops of the two team columns: each team's name over its figure, the side ahead in full
 * ink. A concluded match shows its saved result, a forfeit W and FF.
 */
function ScoreHeads({
  room,
  tally,
  teams,
  bestOf,
}: {
  room: MatchData;
  tally: Tally | null;
  teams: MatchTeam[];
  bestOf: number | null;
}) {
  if (!tally) return null;
  const forfeit = room.forfeit_slot;
  const figures: [string, string] = forfeit
    ? forfeit === 1
      ? ["FF", "W"]
      : ["W", "FF"]
    : room.concluded_at !== null
      ? [String(room.team1_score ?? "–"), String(room.team2_score ?? "–")]
      : [String(tally.team1), String(tally.team2)];
  const lead =
    figures[0] === figures[1]
      ? null
      : Number(figures[0]) > Number(figures[1]) || figures[0] === "W"
        ? 0
        : 1;
  // The wins each team needs, as pips under its figure; a forfeit has none to count.
  const need = bestOf !== null && !forfeit ? firstTo(bestOf) : null;
  return (
    <>
      {teams.map((team, i) => {
        const wins = Number(figures[i]) || 0;
        return (
          <div
            key={team.id}
            className={`sheet-team is-team-${team.color}${lead === i ? " is-ahead" : ""}`}
          >
            <span className="sheet-team-name" title={team.name}>
              {team.name}
            </span>
            <span
              className="sheet-team-figure"
              aria-label={`${team.name}: ${figures[i]}${need !== null ? `, ${need} map gerekli` : ""}`}
            >
              {figures[i]}
            </span>
            {need !== null && (
              <span className="sheet-pips" aria-hidden="true">
                {Array.from({ length: need }, (_, k) => (
                  <span key={k} className={k < wins ? "is-lit" : undefined} />
                ))}
              </span>
            )}
          </div>
        );
      })}
    </>
  );
}

/**
 * Where the match stands against its best of, under the score while it's played: a team on match
 * point, a team that has the wins it needs (the referee still concludes it), or a score the best
 * of can't have. Nothing otherwise; the tiebreaker shows in the pool and the room.
 */
function StandingLine({
  standing,
  score,
  teams,
  room,
  action,
}: {
  standing: Standing;
  score: [number, number];
  teams: MatchTeam[];
  room: MatchData;
  action?: (standing: Standing) => ReactNode;
}) {
  if (standing.kind === "open" || standing.kind === "tiebreaker") return null;
  const bestOf = room.stage?.best_of ?? 0;
  const line = `${score[0]} – ${score[1]}`;
  let tone = "";
  let body: ReactNode;
  switch (standing.kind) {
    case "matchPoint": {
      const team = teams[standing.slots[0] - 1];
      tone = ` is-match-point is-team-${team.color}`;
      body = (
        <>
          <strong className="sheet-standing-title">Maç sayısı</strong>
          <span className="sheet-standing-detail">
            <span className={`sheet-standing-team is-team-${team.color}`}>{team.name}</span> bir map
            daha alırsa kazanır
          </span>
        </>
      );
      break;
    }
    case "won": {
      const team = teams[standing.slot - 1];
      tone = ` is-won is-team-${team.color}`;
      body = (
        <>
          <strong className="sheet-standing-title">{team.name} maçı aldı</strong>
          <span className="sheet-standing-detail">
            {line} · {bestOfLabel(bestOf)} · hakem onaylayınca kesinleşir
          </span>
        </>
      );
      break;
    }
    case "over":
      tone = " is-over";
      body = (
        <>
          <strong className="sheet-standing-title">Skor {bestOfLabel(bestOf)}'u aşıyor</strong>
          <span className="sheet-standing-detail">
            {line} · sayılan map'lere bak, fazladan biri sayılmış olabilir
          </span>
        </>
      );
      break;
  }
  const extra = action?.(standing);
  return (
    <div className={`sheet-standing${tone}`} role="status">
      <span className="sheet-standing-text">{body}</span>
      {extra && <span className="sheet-standing-action">{extra}</span>}
    </div>
  );
}

function MapCell({
  entry,
  name,
  children,
}: {
  entry: PoolEntry;
  name: string | null;
  children?: ReactNode;
}) {
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
function TotalCells({
  teams,
  totals,
  counted,
}: {
  teams: MatchTeam[];
  totals: TeamTotal[];
  counted: boolean;
}) {
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
 * An opened bracket map: one line per player, best score first whatever their team, each with
 * their place on the map, their name and mods under the map's name and their score in their
 * team's column, so the ranking steps across the two columns and adds up to the totals above.
 */
function PlayerRows({ teams, players }: { teams: MatchTeam[]; players: PlayerLine[] }) {
  // playerLines already ranks them by score.
  const ranked = players.flatMap((p) => {
    const team = teams.find((t) => t.id === p.teamId);
    return team ? [{ p, team }] : [];
  });
  const others = players.filter((p) => !teams.some((t) => t.id === p.teamId));
  return (
    <ol className="sheet-players" aria-label="Oyuncu skorları">
      {ranked.map(({ p, team }, i) => (
        <li key={p.key} className="sheet-row is-player">
          <span className="sheet-place">{i + 1}</span>
          <span className="sheet-player">
            <span className={`sheet-player-dot is-team-${team.color}`} aria-hidden="true" />
            <span className="sheet-player-name">{p.name}</span>
            <span className={`sheet-mods${p.mods === "?" || p.mods === "NM" ? " is-plain" : ""}`}>
              {p.mods}
            </span>
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
      ))}
      {others.length > 0 && (
        <li className="sheet-row is-player is-others">
          <span />
          <span className="sheet-sub-note">
            Maçın oyuncusu değil (sayılmadı):{" "}
            {others.map((p) => `${p.name} ${formatScore(p.score)}`).join(", ")}
          </span>
        </li>
      )}
    </ol>
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
                <span
                  className={`sheet-mods${p.mods === "?" || p.mods === "NM" ? " is-plain" : ""}`}
                >
                  {p.mods}
                </span>
                <span className={`sheet-lobby-player-score${p.passed ? "" : " is-failed"}`}>
                  {formatScore(p.score)}
                </span>
              </span>
            ))}
        </li>
      ))}
    </ol>
  );
}
