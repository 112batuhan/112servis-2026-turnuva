import { useMemo, useState, type CSSProperties, type ReactNode } from "react";
import { useSearchParams } from "react-router-dom";
import type { StatCell, StatStanding } from "../../api/types";
import { Icon } from "../../components/Icon";
import { MapLabel, PlayerName, TeamWho, ZValue } from "./StatsParts";
import {
  accuracyUnknown,
  formatAccuracy,
  formatScore,
  formatZ,
  matchesQuery,
  type StatsIndex,
} from "./statsModel";
import "./Standings.css";

type Kind = "players" | "teams";
type View = "table" | "matrix";
type Metric = "z" | "rank" | "score" | "accuracy";

interface Props {
  kind: Kind;
  index: StatsIndex;
  ownTeam: string | null;
}

interface Column {
  id: string;
  label: string;
  title?: string;
  className?: string;
  /** Higher sorts first; left out, the column doesn't sort. */
  value?: (s: StatStanding) => number;
  render: (s: StatStanding) => ReactNode;
}

interface Sort {
  id: string;
  /** "best" is the column's own order (highest first); "reverse" flips it. */
  dir: "best" | "reverse";
}

const RANK_SORT: Sort = { id: "rank", dir: "best" };

/**
 * The stage's standings: every player or team with what they did across the pool. The qualifier
 * ranks teams by the sum of their Z scores, which is the bracket's seeding; a bracket stage by the
 * average Z of the maps played. A table to sort and search, or the same rows against every map.
 */
export function Standings({ kind, index, ownTeam }: Props) {
  const [query, setQuery] = useState("");
  // The matrix is in the URL (?view=matrix), so a link can open it.
  const [params, setParams] = useSearchParams();
  const view: View = params.get("view") === "matrix" ? "matrix" : "table";
  const setView = (v: View) =>
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (v === "matrix") next.set("view", v);
        else next.delete("view");
        return next;
      },
      { replace: true },
    );
  const [metric, setMetric] = useState<Metric>("z");
  const [sort, setSort] = useState<Sort>(RANK_SORT);
  const rows = kind === "teams" ? index.data.stats.teams : index.data.stats.players;
  const q = index.qualifier;
  const poolSize = index.entries.length;

  const isOwn = (s: StatStanding) => s.team_id === ownTeam;
  const who = (s: StatStanding) =>
    kind === "teams" ? (
      <TeamWho team={index.team.get(s.id)} own={isOwn(s)} />
    ) : (
      <PlayerName player={index.player.get(s.id)} index={index} own={isOwn(s)} />
    );

  const columns: Column[] = useMemo(() => {
    const zSum: Column = {
      id: "z",
      label: q ? "Z toplamı" : "Ort. Z",
      title: q ? "Bütün maplerdeki Z'lerin toplamı; eksik mapler 0 puan sayılır" : "Oynanan maplerdeki Z'lerin ortalaması",
      className: "is-z",
      value: (s) => (q ? s.z_sum : s.z_avg),
      render: (s) => <ZValue z={q ? s.z_sum : s.z_avg} size="lg" />,
    };
    const maps: Column = {
      id: "maps",
      label: "Map",
      title: q ? "Oynanan mapler / havuz" : "Oynanan map sayısı",
      className: "is-num",
      value: (s) => s.plays,
      render: (s) => {
        const missing = s.per_map.filter((c) => c.missing).length;
        return (
          <span className="standings-maps">
            <span className="num">{q ? `${s.plays}/${poolSize}` : s.plays}</span>
            {missing > 0 && (
              <span className="standings-missing" title="Lobisi bittiği hâlde skoru olmayan mapler: 0 puan sayıldı">
                {missing} eksik
              </span>
            )}
          </span>
        );
      },
    };
    const score: Column = q
      ? {
          id: "score",
          label: "Toplam skor",
          className: "is-num is-opt",
          value: (s) => s.total_score,
          render: (s) => <span className="num">{formatScore(s.total_score)}</span>,
        }
      : {
          id: "score",
          label: "Ort. skor",
          className: "is-num is-opt",
          value: (s) => s.avg_score,
          render: (s) => <span className="num">{formatScore(s.avg_score)}</span>,
        };
    const accuracy: Column = {
      id: "accuracy",
      label: "Ort. doğruluk",
      className: "is-num is-opt",
      value: (s) => s.avg_accuracy ?? -1,
      render: (s) => (
        <span className="num" title={s.avg_accuracy === null ? accuracyUnknown : undefined}>
          {formatAccuracy(s.avg_accuracy)}
        </span>
      ),
    };
    const record: Column = {
      id: "record",
      label: "Map G–K",
      title: "Takımın kazandığı ve kaybettiği mapler",
      className: "is-num is-opt2",
      value: (s) => s.map_wins - s.map_losses,
      render: (s) => (
        <span className="num">
          {s.map_wins}–{s.map_losses}
        </span>
      ),
    };
    const best: Column = {
      id: "best",
      label: "En iyi map",
      title: "Sahasının en üstünde kaldığı oyun: Z'si ve o map'teki sırası",
      className: "is-opt2",
      value: (s) => s.best?.z ?? -Infinity,
      render: (s) =>
        s.best ? (
          <span className="standings-best">
            <MapLabel entry={index.entry.get(s.best.map_id)} />
            <span className="num">{formatZ(s.best.z)}</span>
            <span className="standings-best-rank">
              {s.best.rank}./{s.best.field}
            </span>
          </span>
        ) : (
          <span className="standings-none">—</span>
        ),
    };
    const out: Column[] = [zSum, maps];
    if (!q) out.push(record);
    out.push(score, accuracy);
    if (kind === "players") {
      out.push({
        id: "team-best",
        label: "Takımda önde",
        title: "Takım arkadaşından yüksek skor yaptığı mapler",
        className: "is-num is-opt2",
        value: (s) => s.team_best,
        render: (s) => <span className="num">{s.team_best}</span>,
      });
    } else if (!q) {
      out.push({
        id: "draft",
        label: "Pick · ban",
        className: "is-num is-opt2",
        value: (s) => s.picks,
        render: (s) => (
          <span className="num">
            {s.picks} · {s.bans}
          </span>
        ),
      });
    }
    out.push(best);
    return out;
  }, [q, kind, index, poolSize]);

  const shown = useMemo(() => {
    const filtered = rows.filter((s) => {
      if (kind === "teams") {
        const team = index.team.get(s.id);
        return matchesQuery(query, team?.name, ...(team?.players.map((p) => p.username) ?? []));
      }
      return matchesQuery(query, index.player.get(s.id)?.username, index.team.get(s.team_id)?.name);
    });
    if (sort.id === "rank") {
      return sort.dir === "best" ? filtered : [...filtered].reverse();
    }
    const value =
      columns.find((c) => c.id === sort.id)?.value ??
      ((s: StatStanding) => s.per_map.find((c) => c.map_id === sort.id)?.z ?? -Infinity);
    const sign = sort.dir === "best" ? 1 : -1;
    return [...filtered].sort((a, b) => sign * (value(b) - value(a)));
  }, [rows, query, sort, columns, kind, index]);

  const toggleSort = (id: string) =>
    setSort((s) => (s.id === id ? { id, dir: s.dir === "best" ? "reverse" : "best" } : { id, dir: "best" }));
  const ariaSort = (id: string): "ascending" | "descending" | undefined =>
    sort.id !== id ? undefined : (id === "rank") === (sort.dir === "best") ? "ascending" : "descending";

  const pending = rows.filter((s) => s.pending).length;

  return (
    <section className="standings" aria-label={kind === "teams" ? "Takım sıralaması" : "Oyuncu sıralaması"}>
      <div className="stats-toolbar">
        <p className="stats-toolbar-note">
          {kind === "teams"
            ? q
              ? "Seeding: takımlar bütün maplerdeki Z'lerinin toplamına göre sıralanır."
              : "Takımlar oynadıkları maplerdeki Z'lerin ortalamasına göre sıralanır."
            : q
              ? "Oyuncular bütün maplerdeki kendi Z'lerinin toplamına göre sıralanır."
              : "Oyuncular oynadıkları maplerdeki kendi Z'lerinin ortalamasına göre sıralanır."}
          {pending > 0 && ` ${pending} ${kind === "teams" ? "takımın" : "oyuncunun"} lobisi henüz bitmedi; eksik mapleri bitince sayılır.`}
        </p>
        <div className="stats-toolbar-controls">
          <div className="bracket-view-tabs" role="group" aria-label="Görünüm">
            <button type="button" className="bracket-view-tab" aria-pressed={view === "table"} onClick={() => setView("table")}>
              Sıralama
            </button>
            <button type="button" className="bracket-view-tab" aria-pressed={view === "matrix"} onClick={() => setView("matrix")}>
              Map tablosu
            </button>
          </div>
          {view === "matrix" && (
            <div className="bracket-view-tabs" role="group" aria-label="Hücrede">
              {(
                [
                  ["z", "Z"],
                  ["rank", "Sıra"],
                  ["score", "Skor"],
                  ["accuracy", "Doğruluk"],
                ] as const
              ).map(([id, label]) => (
                <button key={id} type="button" className="bracket-view-tab" aria-pressed={metric === id} onClick={() => setMetric(id)}>
                  {label}
                </button>
              ))}
            </div>
          )}
          <input
            type="search"
            className="input stats-search"
            placeholder={kind === "teams" ? "Takım ya da oyuncu ara" : "Oyuncu ya da takım ara"}
            aria-label="Ara"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </div>
      </div>

      {rows.length === 0 ? (
        <p className="stats-empty">Henüz sıralanacak bir skor yok.</p>
      ) : view === "table" ? (
        <div className="stats-table-wrap">
          <table className="stats-table standings-table">
            <thead>
              <tr>
                <SortHeader id="rank" label={q && kind === "teams" ? "Seed" : "Sıra"} className="is-rank" sort={ariaSort("rank")} onSort={toggleSort} />
                <th className="is-who">{kind === "teams" ? "Takım" : "Oyuncu"}</th>
                {columns.map((c) => (
                  <SortHeader
                    key={c.id}
                    id={c.id}
                    label={c.label}
                    title={c.title}
                    className={c.className}
                    sort={ariaSort(c.id)}
                    onSort={c.value ? toggleSort : undefined}
                  />
                ))}
              </tr>
            </thead>
            <tbody>
              {shown.map((s) => (
                <tr key={s.id} className={[isOwn(s) && "is-own", s.pending && "is-pending"].filter(Boolean).join(" ") || undefined}>
                  <td className="is-rank">
                    <span className={q && kind === "teams" ? "standings-seed" : undefined}>{s.rank ?? "—"}</span>
                  </td>
                  <td className="is-who">
                    <span className="standings-who">
                      {who(s)}
                      {s.pending && <span className="standings-pending">lobisi sürüyor</span>}
                    </span>
                  </td>
                  {columns.map((c) => (
                    <td key={c.id} className={c.className}>
                      {c.render(s)}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : (
        <Matrix
          rows={shown}
          index={index}
          metric={metric}
          who={who}
          isOwn={isOwn}
          sortedBy={sort.id}
          onSort={toggleSort}
          total={(s) => (q ? s.z_sum : s.z_avg)}
          whoLabel={`${kind === "teams" ? "Takım" : "Oyuncu"} · ${q ? "Z toplamı" : "Ort. Z"}`}
        />
      )}
    </section>
  );
}

function SortHeader({
  id,
  label,
  title,
  className,
  sort,
  onSort,
}: {
  id: string;
  label: string;
  title?: string;
  className?: string;
  sort: "ascending" | "descending" | undefined;
  onSort?: (id: string) => void;
}) {
  return (
    <th className={className} aria-sort={sort} title={title}>
      {onSort ? (
        <button type="button" className={sort ? "stats-sort is-sorted" : "stats-sort"} onClick={() => onSort(id)}>
          {label}
          <Icon name={sort === "ascending" ? "arrowUp" : "arrowDown"} size={12} className="stats-sort-mark" />
        </button>
      ) : (
        label
      )}
    </th>
  );
}

interface MatrixProps {
  rows: StatStanding[];
  index: StatsIndex;
  metric: Metric;
  who: (s: StatStanding) => ReactNode;
  isOwn: (s: StatStanding) => boolean;
  sortedBy: string;
  onSort: (id: string) => void;
  total: (s: StatStanding) => number;
  whoLabel: string;
}

/**
 * The rows against every map of the pool: each cell what they did there (the metric picked),
 * shaded brighter the further above the map's field they were. A missing qualifier map shows the
 * 0 it was scored as.
 */
function Matrix({ rows, index, metric, who, isOwn, sortedBy, onSort, total, whoLabel }: MatrixProps) {
  return (
    <div className="stats-matrix-wrap">
      <table className="stats-table stats-matrix">
        <thead>
          <tr>
            <th className="is-rank">Sıra</th>
            <th className="stats-matrix-who">{whoLabel}</th>
            {index.entries.map((e) => (
              <th key={e.map.id} className="stats-matrix-map" aria-sort={sortedBy === e.map.id ? "descending" : undefined}>
                <button
                  type="button"
                  className={sortedBy === e.map.id ? "stats-sort is-sorted" : "stats-sort"}
                  onClick={() => onSort(e.map.id)}
                  title={`${e.map.title} [${e.map.version}]: bu map'teki Z'ye göre sırala`}
                >
                  <MapLabel entry={e} />
                </button>
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((s) => (
            <tr key={s.id} className={isOwn(s) ? "is-own" : undefined}>
              <td className="is-rank">{s.rank ?? "—"}</td>
              <td className="stats-matrix-who">
                <span className="stats-matrix-who-line">
                  {who(s)}
                  <ZValue z={total(s)} />
                </span>
              </td>
              {index.entries.map((e) => (
                <MatrixCell key={e.map.id} cell={s.per_map.find((c) => c.map_id === e.map.id)} metric={metric} />
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function MatrixCell({ cell, metric }: { cell: StatCell | undefined; metric: Metric }) {
  if (!cell) {
    return (
      <td className="stats-matrix-cell is-empty">
        <span aria-label="Oynanmadı">·</span>
      </td>
    );
  }
  if (cell.missing) {
    return (
      <td className="stats-matrix-cell is-missing" title={`Skoru yok: 0 puan sayıldı (Z ${formatZ(cell.z)})`}>
        {metric === "z" ? formatZ(cell.z) : "0"}
      </td>
    );
  }
  // Brightest at two and a half deviations above the field.
  const heat = Math.max(0, Math.min(1, cell.z / 2.5));
  const text =
    metric === "z"
      ? formatZ(cell.z)
      : metric === "rank"
        ? `${cell.rank ?? "—"}.`
        : metric === "score"
          ? formatScore(cell.score ?? 0)
          : formatAccuracy(cell.accuracy);
  return (
    <td
      className={cell.z < 0 ? "stats-matrix-cell is-below" : "stats-matrix-cell"}
      style={{ "--heat": heat } as CSSProperties}
      title={`Z ${formatZ(cell.z)} · ${formatScore(cell.score ?? 0)} · ${formatAccuracy(cell.accuracy)}${cell.plays > 1 ? ` · ${cell.plays} oyun` : ""}`}
    >
      {text}
    </td>
  );
}
