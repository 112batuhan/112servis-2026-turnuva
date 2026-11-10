import { useMemo, useState } from "react";
import type { StatPlayerPlay, StatTeamPlay } from "../../api/types";
import { MapLabel, MatchCode, PlayerName, TeamWho, ZValue } from "./StatsParts";
import {
  accuracyUnknown,
  formatAccuracy,
  formatMods,
  formatScore,
  matchesQuery,
  type StatsIndex,
} from "./statsModel";
import "./MapBoard.css";

interface Props {
  kind: "players" | "teams";
  index: StatsIndex;
  ownTeam: string | null;
  mapId: string | null;
  onMap: (mapId: string) => void;
}

interface PlayerRow {
  play: StatPlayerPlay;
  team: StatTeamPlay;
}

/**
 * One map's leaderboard: every player's (or team's) play on it, best first, with its accuracy,
 * mods and Z against everyone who played it. The pool's maps run along the top to switch.
 */
export function MapBoard({ kind, index, ownTeam, mapId, onMap }: Props) {
  const [query, setQuery] = useState("");
  const played = index.entries.filter((e) => (index.map.get(e.map.id)?.teams.count ?? 0) > 0);
  const entry = index.entries.find((e) => e.map.id === mapId) ?? played[0] ?? index.entries[0];
  const stat = entry ? index.map.get(entry.map.id) : undefined;

  const teamRows = useMemo(
    () =>
      index.data.stats.plays
        .filter((p) => p.map_id === entry?.map.id)
        .sort((a, b) => a.rank - b.rank || b.score - a.score),
    [index, entry],
  );
  const playerRows: PlayerRow[] = useMemo(
    () =>
      teamRows
        .flatMap((team) => team.players.map((play) => ({ play, team })))
        .sort((a, b) => a.play.rank - b.play.rank || b.play.score - a.play.score),
    [teamRows],
  );

  if (!entry) return <p className="stats-empty">Bu stage'in havuzunda map yok.</p>;
  const field = kind === "players" ? stat?.players : stat?.teams;
  const name = (userId: string) => index.player.get(userId)?.username;
  const teamName = (teamId: string) => index.team.get(teamId)?.name;

  return (
    <section className="map-board" aria-label={kind === "players" ? "Map'te oyuncu skorları" : "Map'te takım skorları"}>
      <nav className="map-board-picker" aria-label="Mapler">
        {index.entries.map((e) => {
          const count = index.map.get(e.map.id)?.teams.count ?? 0;
          return (
            <button
              key={e.map.id}
              type="button"
              className={count === 0 ? "map-board-pick is-unplayed" : "map-board-pick"}
              aria-pressed={e.map.id === entry.map.id}
              onClick={() => onMap(e.map.id)}
              title={`${e.map.artist} - ${e.map.title} [${e.map.version}]${count === 0 ? " · oynanmadı" : ""}`}
            >
              <MapLabel entry={e} />
            </button>
          );
        })}
      </nav>

      <header className="map-board-head">
        {entry.map.cover_url && <img className="map-board-cover" src={entry.map.cover_url} alt="" loading="lazy" />}
        <div className="map-board-title">
          <h3>
            <MapLabel entry={entry} />
            <a href={`https://osu.ppy.sh/b/${entry.map.beatmap_id}`} target="_blank" rel="noopener noreferrer">
              {entry.map.title}
            </a>
          </h3>
          <p className="map-board-sub">
            {entry.map.artist} · [{entry.map.version}] · <span className="num">{entry.map.star_rating.toFixed(2)}</span>★
          </p>
        </div>
        {field && field.count > 0 && (
          <dl className="map-board-field">
            <div>
              <dt>{kind === "players" ? "Oyuncu" : "Takım"}</dt>
              <dd>{field.count}</dd>
            </div>
            <div>
              <dt>Ortalama</dt>
              <dd>{formatScore(field.mean)}</dd>
            </div>
            <div>
              <dt>Standart sapma</dt>
              <dd>{formatScore(field.sd)}</dd>
            </div>
            {!index.qualifier && stat && (
              <div>
                <dt>Pick · ban</dt>
                <dd>
                  {stat.picks} · {stat.bans}
                </dd>
              </div>
            )}
          </dl>
        )}
      </header>

      <div className="stats-toolbar">
        <p className="stats-toolbar-note">
          {kind === "players"
            ? "Map'teki her oyuncu skoru. Z, bu map'i oynayan bütün oyunculara göre."
            : index.qualifier
              ? "Her takımın bu map'teki skoru (son denemesi). Z, bu map'i oynayan bütün takımlara göre."
              : "Her takım skoru; bir takım map'i iki maçta oynadıysa ikisi de listelenir."}
        </p>
        <input
          type="search"
          className="input input-sm stats-search"
          placeholder="Oyuncu ya da takım ara"
          aria-label="Ara"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
      </div>

      {teamRows.length === 0 ? (
        <p className="stats-empty">Bu map henüz oynanmadı.</p>
      ) : (
        <div className="stats-table-wrap">
          <table className="stats-table map-board-table">
            <thead>
              <tr>
                <th className="is-rank">Sıra</th>
                <th className="is-who">{kind === "players" ? "Oyuncu" : "Takım"}</th>
                <th className="is-num">Skor</th>
                <th className="is-num is-opt">Accuracy</th>
                {kind === "players" && <th className="is-opt">Mod</th>}
                {kind === "teams" && !index.qualifier && <th className="is-opt2">Sonuç</th>}
                <th className="is-z">Z</th>
                <th className="is-opt2">Maç</th>
              </tr>
            </thead>
            <tbody>
              {kind === "players"
                ? playerRows
                    .filter((r) => matchesQuery(query, name(r.play.user_id), teamName(r.team.team_id)))
                    .map((r) => (
                      <tr
                        key={`${r.team.match_id}:${r.play.user_id}:${r.play.score}`}
                        className={r.team.team_id === ownTeam ? "is-own" : undefined}
                      >
                        <td className="is-rank">{r.play.rank}</td>
                        <td className="is-who">
                          <PlayerName
                            player={index.player.get(r.play.user_id)}
                            index={index}
                            own={r.team.team_id === ownTeam}
                          />
                        </td>
                        <td className="is-num is-score">
                          {formatScore(r.play.score)}
                          {!r.play.passed && <span className="stats-fail">fail</span>}
                        </td>
                        <td className="is-num is-opt" title={r.play.accuracy === null ? accuracyUnknown : undefined}>
                          {formatAccuracy(r.play.accuracy)}
                        </td>
                        <td className="is-opt stats-mods">{formatMods(r.play.mods)}</td>
                        <td className="is-z">
                          <ZValue z={r.play.z} />
                        </td>
                        <td className="is-opt2">
                          <MatchCode matchId={r.team.match_id} index={index} />
                        </td>
                      </tr>
                    ))
                : teamRows
                    .filter((p) => {
                      const team = index.team.get(p.team_id);
                      return matchesQuery(query, team?.name, ...(team?.players.map((x) => x.username) ?? []));
                    })
                    .map((p) => (
                      <tr key={`${p.match_id}:${p.team_id}:${p.score}`} className={p.team_id === ownTeam ? "is-own" : undefined}>
                        <td className="is-rank">{p.rank}</td>
                        <td className="is-who">
                          <TeamWho team={index.team.get(p.team_id)} own={p.team_id === ownTeam} />
                          <span className="map-board-split">
                            {p.players.map((pp) => (
                              <span key={pp.user_id}>
                                {name(pp.user_id)} <span className="num">{formatScore(pp.score)}</span>
                                <span className="map-board-split-mods">{formatMods(pp.mods)}</span>
                              </span>
                            ))}
                          </span>
                        </td>
                        <td className="is-num is-score">{formatScore(p.score)}</td>
                        <td className="is-num is-opt" title={p.accuracy === null ? accuracyUnknown : undefined}>
                          {formatAccuracy(p.accuracy)}
                        </td>
                        {!index.qualifier && (
                          <td className="is-opt2">
                            <span className={`map-board-result is-${p.won === null ? "tie" : p.won ? "won" : "lost"}`}>
                              {p.won === null ? "Berabere" : p.won ? "Kazandı" : "Kaybetti"}
                            </span>
                            {p.picked && <span className="map-board-picked">pick</span>}
                          </td>
                        )}
                        <td className="is-z">
                          <ZValue z={p.z} />
                        </td>
                        <td className="is-opt2">
                          <MatchCode matchId={p.match_id} index={index} />
                        </td>
                      </tr>
                    ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
