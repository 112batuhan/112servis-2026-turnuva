import { useMemo, useState, type CSSProperties, type ReactNode } from "react";
import { useSearchParams } from "react-router-dom";
import type { StatCell, StatDraftTally, StatStanding } from "../../api/types";
import { Icon } from "../../components/Icon";
import { MapLabel, PanelTools, PlayerName, TeamWho, ZValue } from "./StatsParts";
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
  // The matrix's pick and ban dots are off until asked for, and in the URL (?drafts=1) like the
  // matrix itself.
  const showDrafts = params.get("drafts") === "1";
  const toggleDrafts = () =>
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        if (showDrafts) next.delete("drafts");
        else next.set("drafts", "1");
        return next;
      },
      { replace: true },
    );
  const [metric, setMetric] = useState<Metric>("score");
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
      label: "Oynanan mapler",
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
      label: "Ort. accuracy",
      className: "is-num is-opt",
      value: (s) => s.avg_accuracy ?? -1,
      render: (s) => (
        <span className="num" title={s.avg_accuracy === null ? accuracyUnknown : undefined}>
          {formatAccuracy(s.avg_accuracy)}
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
    const out: Column[] = [zSum, maps, score, accuracy];
    if (kind === "players") {
      out.push({
        id: "team-best",
        label: "Takımda önde",
        title: "Takım arkadaşından yüksek skor yaptığı mapler",
        className: "is-num is-opt2",
        value: (s) => s.team_best,
        render: (s) => <span className="num">{s.team_best}</span>,
      });
    }
    out.push(best);
    return out;
  }, [q, kind, index, poolSize]);

  const filtered = useMemo(
    () =>
      rows.filter((s) => {
        if (kind === "teams") {
          const team = index.team.get(s.id);
          return matchesQuery(query, team?.name, ...(team?.players.map((p) => p.username) ?? []));
        }
        return matchesQuery(query, index.player.get(s.id)?.username, index.team.get(s.team_id)?.name);
      }),
    [rows, query, kind, index],
  );

  // The standings table's order.
  const shown = useMemo(() => {
    if (sort.id === "rank") {
      return sort.dir === "best" ? filtered : [...filtered].reverse();
    }
    const value = columns.find((c) => c.id === sort.id)?.value;
    if (!value) return filtered;
    const sign = sort.dir === "best" ? 1 : -1;
    return [...filtered].sort((a, b) => sign * (value(b) - value(a)));
  }, [filtered, sort, columns]);

  // The matrix keeps its own order: by one map, on whatever its cells show, best first (level
  // rows keep the standings' order); a second click on the map hands them back to the standings.
  const [mapSort, setMapSort] = useState<string | null>(null);
  const matrixRows = useMemo(() => {
    if (!mapSort) return filtered;
    const value = (s: StatStanding) => cellValue(s.per_map.find((c) => c.map_id === mapSort), metric);
    return [...filtered].sort((a, b) => value(b) - value(a));
  }, [filtered, mapSort, metric]);

  const toggleSort = (id: string) =>
    setSort((s) => (s.id === id ? { id, dir: s.dir === "best" ? "reverse" : "best" } : { id, dir: "best" }));
  const toggleMapSort = (id: string) => setMapSort((m) => (m === id ? null : id));
  const ariaSort = (id: string): "ascending" | "descending" | undefined =>
    sort.id !== id ? undefined : (id === "rank") === (sort.dir === "best") ? "ascending" : "descending";

  const drafted = rows.some((s) => s.drafts.length > 0);

  return (
    <section className="standings" aria-label={kind === "teams" ? "Takım sıralaması" : "Oyuncu sıralaması"}>
      <PanelTools>
        <input
          type="search"
          className="input input-sm stats-search"
          placeholder="Oyuncu ya da takım ara"
          aria-label="Ara"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
        />
        <div className="bracket-view-tabs stats-view-switch" role="group" aria-label="Görünüm">
          <button type="button" className="bracket-view-tab" aria-pressed={view === "table"} onClick={() => setView("table")}>
            Sıralama
          </button>
          <button type="button" className="bracket-view-tab" aria-pressed={view === "matrix"} onClick={() => setView("matrix")}>
            Map tablosu
          </button>
        </div>
      </PanelTools>

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
        <>
          {/* What the matrix's cells show, over the matrix itself. */}
          <div className="standings-matrix-bar">
            <div className="bracket-view-tabs" role="group" aria-label="Hücrede">
              {(
                [
                  ["score", "Skor"],
                  ["accuracy", "Accuracy"],
                  ["z", "Z"],
                  ["rank", "Sıra"],
                ] as const
              ).map(([id, label]) => (
                <button key={id} type="button" className="bracket-view-tab" aria-pressed={metric === id} onClick={() => setMetric(id)}>
                  {label}
                </button>
              ))}
            </div>
            {drafted && (
              <div className="bracket-view-tabs">
                <button type="button" className="bracket-view-tab draft-toggle" aria-pressed={showDrafts} onClick={toggleDrafts}>
                  <span className="draft-toggle-dots" aria-hidden="true">
                    <span className="draft-dot is-team" />
                    <span className="draft-dot is-opponent" />
                  </span>
                  Pick ve banlar
                </button>
              </div>
            )}
            {drafted && showDrafts && <DraftLegend kind={kind} />}
          </div>
          <Matrix
            rows={matrixRows}
            index={index}
            metric={metric}
            showDrafts={showDrafts}
            who={who}
            isOwn={isOwn}
            sortedBy={mapSort}
            onSort={toggleMapSort}
            total={(s) => (q ? s.z_sum : s.z_avg)}
            whoLabel={`${kind === "teams" ? "Takım" : "Oyuncu"} · ${q ? "Z toplamı" : "Ort. Z"}`}
          />
        </>
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

/** What can be drafted on a map in the row's team's matches: by the team itself, or by its
 * opponents. A played map was picked, so its figure takes the picker's colour; a banned one was
 * never played, so its empty cell takes a dot in the banner's. The words keep everything. */
const DRAFT_MARKS = [
  { key: "picked", side: "team", ban: false, label: "Pick'ledi" },
  { key: "banned", side: "team", ban: true, label: "Banladı" },
  { key: "picked_against", side: "opponent", ban: false, label: "Rakibi pick'ledi" },
  { key: "banned_against", side: "opponent", ban: true, label: "Rakibi banladı" },
] as const;

type DraftSide = (typeof DRAFT_MARKS)[number]["side"];

interface DraftMarks {
  /** Who picked it, the team first should both have (in different matches). */
  picker: DraftSide | null;
  /** Who banned it, the team first. */
  banners: DraftSide[];
  text: string;
}

/** A cell's marks and their words, or null when nobody drafted the map against the row. */
function draftMarks(draft: StatDraftTally | undefined): DraftMarks | null {
  if (!draft) return null;
  const on = DRAFT_MARKS.filter((m) => draft[m.key] > 0);
  if (on.length === 0) return null;
  return {
    picker: on.find((m) => !m.ban)?.side ?? null,
    banners: on.filter((m) => m.ban).map((m) => m.side),
    text: on.map((m) => (draft[m.key] > 1 ? `${m.label} ×${draft[m.key]}` : m.label)).join(" · "),
  };
}

/** A ban: one dot per side that banned the map, in place of the empty cell's figure. */
function DraftDots({ sides }: { sides: DraftSide[] }) {
  return (
    <span className="draft-dots" aria-hidden="true">
      {sides.map((side) => (
        <span key={side} className={`draft-dot is-${side}`} />
      ))}
    </span>
  );
}

/** What the two colours mean: the row's team's picks and bans, and its opponents'. */
function DraftLegend({ kind }: { kind: Kind }) {
  const sides = [
    { side: "team", label: kind === "teams" ? "Takımın pick ve banları" : "Takımının pick ve banları" },
    { side: "opponent", label: "Rakibinin pick ve banları" },
  ];
  return (
    <div className="draft-legend" role="group" aria-label="Pick ve ban renkleri">
      {sides.map((s) => (
        <span key={s.side} className="draft-legend-item">
          <span className={`draft-dot is-${s.side}`} aria-hidden="true" />
          {s.label}
        </span>
      ))}
      <span className="draft-legend-note">Renkli skor: pick'lenip oynanmış map · nokta: banlanmış map</span>
    </div>
  );
}

/** "Sort by …" in Turkish takes the dative: skora, accuracy'ye, Z'ye, sıraya. */
const METRIC_DATIVE: Record<Metric, string> = {
  score: "skora",
  accuracy: "accuracy'ye",
  z: "Z'ye",
  rank: "sıraya",
};

/** A matrix cell as a sort key, higher first: its score, accuracy or Z, or its place turned
 * round so first comes first. A map the row has no figure for sorts last; a missing qualifier
 * map counts as the 0 it was scored as. */
function cellValue(cell: StatCell | undefined, metric: Metric): number {
  if (!cell) return -Infinity;
  switch (metric) {
    case "score":
      return cell.score ?? 0;
    case "accuracy":
      return cell.accuracy ?? -Infinity;
    case "z":
      return cell.z;
    case "rank":
      return cell.rank === null ? -Infinity : -cell.rank;
  }
}

interface MatrixProps {
  rows: StatStanding[];
  index: StatsIndex;
  metric: Metric;
  showDrafts: boolean;
  who: (s: StatStanding) => ReactNode;
  isOwn: (s: StatStanding) => boolean;
  /** The map the rows are sorted by, on the cells' metric; null for the standings' order. */
  sortedBy: string | null;
  onSort: (id: string) => void;
  total: (s: StatStanding) => number;
  whoLabel: string;
}

/**
 * The rows against every map of the pool: each cell what they did there (the metric picked),
 * shaded brighter the further above the map's field they were. A missing qualifier map shows the
 * 0 it was scored as.
 */
function Matrix({ rows, index, metric, showDrafts, who, isOwn, sortedBy, onSort, total, whoLabel }: MatrixProps) {
  return (
    <div className="stats-matrix-wrap">
      <table className="stats-table stats-matrix">
        <thead>
          <tr>
            <th className="is-rank">Sıra</th>
            <th className="stats-matrix-who">{whoLabel}</th>
            {index.entries.map((e) => (
              <th
                key={e.map.id}
                className="stats-matrix-map"
                aria-sort={sortedBy !== e.map.id ? undefined : metric === "rank" ? "ascending" : "descending"}
              >
                <button
                  type="button"
                  className={sortedBy === e.map.id ? "stats-sort is-sorted" : "stats-sort"}
                  onClick={() => onSort(e.map.id)}
                  title={`${e.map.title} [${e.map.version}]: ${
                    sortedBy === e.map.id ? "sıralamaya dön" : `bu map'teki ${METRIC_DATIVE[metric]} göre sırala`
                  }`}
                >
                  <MapLabel entry={e} />
                  <Icon name={metric === "rank" ? "arrowUp" : "arrowDown"} size={12} className="stats-sort-mark" />
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
                <MatrixCell
                  key={e.map.id}
                  cell={s.per_map.find((c) => c.map_id === e.map.id)}
                  draft={showDrafts ? s.drafts.find((d) => d.map_id === e.map.id) : undefined}
                  metric={metric}
                />
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function MatrixCell({ cell, draft, metric }: { cell: StatCell | undefined; draft: StatDraftTally | undefined; metric: Metric }) {
  const marks = draftMarks(draft);
  const said = marks && <span className="sr-only">{`, ${marks.text}`}</span>;
  if (!cell) {
    return (
      <td className="stats-matrix-cell is-empty" title={marks?.text}>
        {marks && marks.banners.length > 0 ? (
          <DraftDots sides={marks.banners} />
        ) : (
          <span aria-label="Oynanmadı">·</span>
        )}
        {said}
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
      className={["stats-matrix-cell", cell.z < 0 && "is-below", marks?.picker && `picked-by-${marks.picker}`]
        .filter(Boolean)
        .join(" ")}
      style={{ "--heat": heat } as CSSProperties}
      title={`Z ${formatZ(cell.z)} · ${formatScore(cell.score ?? 0)} · ${formatAccuracy(cell.accuracy)}${cell.plays > 1 ? ` · ${cell.plays} oyun` : ""}${marks ? `\n${marks.text}` : ""}`}
    >
      {text}
      {said}
    </td>
  );
}
