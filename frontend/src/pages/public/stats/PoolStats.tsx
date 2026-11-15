import { useMemo, useState } from "react";
import type { StatMap } from "../../api/types";
import type { PoolEntry } from "../../lib/matchPool";
import { MapLabel, MatchCode, PanelTools, PlayerName, TeamName } from "./StatsParts";
import {
  accuracyUnknown,
  formatAccuracy,
  formatMods,
  formatScore,
  formatShare,
  type StatsIndex,
} from "./statsModel";
import "./PoolStats.css";

type Order = "pool" | "picks" | "bans";

interface Props {
  index: StatsIndex;
  ownTeam: string | null;
  /** Opens the map's player leaderboard. */
  onOpenMap: (mapId: string) => void;
}

/**
 * The pool, map by map: how often each was picked and banned (a bracket stage) or played (the
 * qualifier), the field's average, and the best player's and team's scores on it.
 */
export function PoolStats({ index, ownTeam, onOpenMap }: Props) {
  const [order, setOrder] = useState<Order>("pool");
  const bracket = !index.qualifier;
  const matches = index.data.stats.matches_played;

  // By category in pool order, or one list by picks or bans.
  const groups = useMemo(() => {
    const rows = index.entries.map((entry) => ({ entry, stat: index.map.get(entry.map.id) }));
    if (order !== "pool") {
      const key = (s: StatMap | undefined) =>
        order === "picks" ? (s?.picks ?? 0) : (s?.bans ?? 0);
      return [
        {
          name: null as string | null,
          color: null as string | null,
          rows: [...rows].sort((a, b) => key(b.stat) - key(a.stat)),
        },
      ];
    }
    const byCategory: { name: string | null; color: string | null; rows: typeof rows }[] = [];
    for (const category of index.data.pool.categories) {
      const own = rows.filter((r) =>
        index.data.pool.slots.some(
          (s) => s.id === r.entry.map.slot_id && s.category_id === category.id,
        ),
      );
      if (own.length > 0)
        byCategory.push({ name: category.name, color: category.color, rows: own });
    }
    return byCategory;
  }, [index, order]);

  return (
    <section className="pool-stats" aria-label="Mappool istatistikleri">
      {bracket && (
        <PanelTools>
          <div className="bracket-view-tabs" role="group" aria-label="Sıralama">
            {(
              [
                ["pool", "Havuz sırası"],
                ["picks", "En çok pick"],
                ["bans", "En çok ban"],
              ] as const
            ).map(([id, label]) => (
              <button
                key={id}
                type="button"
                className="bracket-view-tab"
                aria-pressed={order === id}
                onClick={() => setOrder(id)}
              >
                {label}
              </button>
            ))}
          </div>
        </PanelTools>
      )}

      <div className={bracket ? "pool-stats-sheet is-bracket" : "pool-stats-sheet"}>
        <div className="pool-stats-head" aria-hidden="true">
          <span>Map</span>
          <span />
          {bracket ? (
            <>
              <span>Pick</span>
              <span>Ban</span>
              <span title="Map'i pick eden takımın onu kazandığı oyunlar">
                Pick'leyenlerin kazanma oranı
              </span>
            </>
          ) : (
            <span>Oynayan</span>
          )}
          <span>Ortalama takım skoru</span>
          <span>En iyi oyuncu</span>
          <span>En iyi takım</span>
        </div>
        {groups.map((g) => (
          <div key={g.name ?? "all"} className="pool-stats-group">
            {g.name && (
              <h3 className="pool-stats-category" style={{ borderTopColor: g.color ?? undefined }}>
                {g.name}
                <span className="pool-stats-category-count">{g.rows.length} map</span>
              </h3>
            )}
            <ol className="pool-stats-rows">
              {g.rows.map(({ entry, stat }) => (
                <PoolRow
                  key={entry.map.id}
                  entry={entry}
                  stat={stat}
                  index={index}
                  ownTeam={ownTeam}
                  matches={matches}
                  onOpen={() => onOpenMap(entry.map.id)}
                />
              ))}
            </ol>
          </div>
        ))}
      </div>
    </section>
  );
}

interface RowProps {
  entry: PoolEntry;
  stat: StatMap | undefined;
  index: StatsIndex;
  ownTeam: string | null;
  matches: number;
  onOpen: () => void;
}

function PoolRow({ entry, stat, index, ownTeam, matches, onOpen }: RowProps) {
  const { map } = entry;
  const bracket = !index.qualifier;
  const bestPlayer = stat?.best_player;
  const bestTeam = stat?.best_team;
  const played = stat?.teams.count ?? 0;
  return (
    <li className={played === 0 ? "pool-stats-row is-unplayed" : "pool-stats-row"}>
      <span className="pool-stats-label">
        <MapLabel entry={entry} />
      </span>
      <span className="pool-stats-map">
        {map.cover_url ? (
          <img className="pool-stats-cover" src={map.cover_url} alt="" loading="lazy" />
        ) : (
          <span className="pool-stats-cover" />
        )}
        <span className="pool-stats-title">
          <a
            href={`https://osu.ppy.sh/b/${map.beatmap_id}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            {map.title}
          </a>
          <span className="pool-stats-sub">
            {map.artist} · [{map.version}]
          </span>
        </span>
        <span className="pool-stats-sr" title="Yıldız (modlarla)">
          <span className="num">{map.star_rating.toFixed(2)}</span>★
        </span>
      </span>

      {bracket ? (
        <>
          <Rate area="pick" label="Pick" count={stat?.picks ?? 0} of={matches} />
          <Rate area="ban" label="Ban" count={stat?.bans ?? 0} of={matches} quiet />
          <span className="pool-stats-cell is-picker" data-label="Pick'leyenlerin kazanma oranı">
            {stat && stat.picker_plays > 0 ? (
              <>
                <span className="pool-stats-figure">
                  {stat.picker_wins}/{stat.picker_plays}
                </span>
                <span className="pool-stats-meta">
                  {formatShare(stat.picker_wins, stat.picker_plays)}
                </span>
              </>
            ) : (
              <span className="pool-stats-none">—</span>
            )}
          </span>
        </>
      ) : (
        <span className="pool-stats-cell is-played" data-label="Oynayan">
          <span className="pool-stats-figure">{played}</span>
          <span className="pool-stats-meta">takım</span>
        </span>
      )}

      <span className="pool-stats-cell is-avg" data-label="Ortalama takım skoru">
        {played > 0 && stat ? (
          <>
            <span className="pool-stats-figure">{formatScore(stat.teams.mean)}</span>
            <span
              className="pool-stats-meta"
              title="Standart sapma ve oyuncuların ortalama doğruluğu"
            >
              σ {formatScore(stat.teams.sd)} · {formatAccuracy(stat.avg_accuracy)}
            </span>
          </>
        ) : (
          <span className="pool-stats-none">Oynanmadı</span>
        )}
      </span>

      <span className="pool-stats-cell pool-stats-best is-best-player" data-label="En iyi oyuncu">
        {bestPlayer ? (
          <>
            <PlayerName
              player={index.player.get(bestPlayer.user_id)}
              index={index}
              own={bestPlayer.team_id === ownTeam}
            />
            <span className="pool-stats-best-score">
              <span className="pool-stats-figure">{formatScore(bestPlayer.score)}</span>
              <span
                className="pool-stats-meta"
                title={bestPlayer.accuracy === null ? accuracyUnknown : undefined}
              >
                {formatAccuracy(bestPlayer.accuracy)} · {formatMods(bestPlayer.mods)} ·{" "}
                <MatchCode matchId={bestPlayer.match_id} index={index} />
              </span>
            </span>
          </>
        ) : (
          <span className="pool-stats-none">—</span>
        )}
      </span>

      <span className="pool-stats-cell pool-stats-best is-best-team" data-label="En iyi takım">
        {bestTeam && index.team.get(bestTeam.team_id) ? (
          <>
            <TeamName team={index.team.get(bestTeam.team_id)!} own={bestTeam.team_id === ownTeam} />
            <span className="pool-stats-best-score">
              <span className="pool-stats-figure">{formatScore(bestTeam.score)}</span>
              <span
                className="pool-stats-meta"
                title={bestTeam.accuracy === null ? accuracyUnknown : undefined}
              >
                {formatAccuracy(bestTeam.accuracy)} ·{" "}
                <MatchCode matchId={bestTeam.match_id} index={index} />
              </span>
            </span>
          </>
        ) : (
          <span className="pool-stats-none">—</span>
        )}
      </span>

      {played > 0 && (
        <button type="button" className="btn btn-secondary btn-sm pool-stats-open" onClick={onOpen}>
          Skorlar
        </button>
      )}
    </li>
  );
}

/** How many of the stage's matches picked (or banned) the map: the figure, its share and a bar. */
function Rate({
  area,
  label,
  count,
  of,
  quiet = false,
}: {
  area: "pick" | "ban";
  label: string;
  count: number;
  of: number;
  quiet?: boolean;
}) {
  const ratio = of > 0 ? Math.min(1, count / of) : 0;
  return (
    <span
      className={`pool-stats-cell pool-stats-rate is-${area}${quiet ? " is-quiet" : ""}`}
      data-label={label}
    >
      <span className="pool-stats-rate-line">
        <span className="pool-stats-figure">{count}</span>
        <span className="pool-stats-meta">{formatShare(count, of)}</span>
      </span>
      <span className="pool-stats-rate-bar" aria-hidden="true">
        <span style={{ width: `${ratio * 100}%` }} />
      </span>
    </span>
  );
}
