import type { Player, StageStats, StatMap, StatMatch, Team } from "../../api/types";
import { poolEntries, type PoolEntry } from "../../lib/matchPool";

export type StatsTab = "pool" | "map-players" | "map-teams" | "players" | "teams";

export const TABS: { id: StatsTab; label: string }[] = [
  { id: "pool", label: "Mappool" },
  { id: "map-players", label: "Oyuncu skorları" },
  { id: "map-teams", label: "Takım skorları" },
  { id: "players", label: "Oyuncular" },
  { id: "teams", label: "Takımlar" },
];

export const isTab = (value: string | null): value is StatsTab => TABS.some((t) => t.id === value);

export interface StatPlayer extends Player {
  team_id: string;
}

/** Everything a stage's statistics refer to by id, looked up once. */
export interface StatsIndex {
  data: StageStats;
  qualifier: boolean;
  /** The pool in category and slot order, with each map's label ("NM1", "TB"). */
  entries: PoolEntry[];
  entry: Map<string, PoolEntry>;
  map: Map<string, StatMap>;
  team: Map<string, Team>;
  player: Map<string, StatPlayer>;
  match: Map<string, StatMatch>;
}

export function buildIndex(data: StageStats): StatsIndex {
  const entries = poolEntries(data.pool);
  const player = new Map<string, StatPlayer>();
  for (const t of data.teams) for (const p of t.players) player.set(p.id, { ...p, team_id: t.id });
  return {
    data,
    qualifier: data.pool.is_qualifier,
    entries,
    entry: new Map(entries.map((e) => [e.map.id, e])),
    map: new Map(data.stats.maps.map((m) => [m.map_id, m])),
    team: new Map(data.teams.map((t) => [t.id, t])),
    player,
    match: new Map(data.matches.map((m) => [m.id, m])),
  };
}

const integer = new Intl.NumberFormat("tr-TR", { maximumFractionDigits: 0 });
const percent = new Intl.NumberFormat("tr-TR", { style: "percent", minimumFractionDigits: 2, maximumFractionDigits: 2 });
const share = new Intl.NumberFormat("tr-TR", { style: "percent", maximumFractionDigits: 0 });
const decimal = new Intl.NumberFormat("tr-TR", { minimumFractionDigits: 2, maximumFractionDigits: 2 });

/** 1.234.567 */
export const formatScore = (n: number) => integer.format(Math.round(n));

/** %98,76; a dash until osu!'s record has been read. */
export const formatAccuracy = (a: number | null) => (a === null ? "—" : percent.format(a));

/** %31 */
export const formatShare = (part: number, whole: number) => (whole > 0 ? share.format(part / whole) : "—");

/** +1,23 / −0,45, with a real minus sign. */
export function formatZ(z: number): string {
  const rounded = Math.round(z * 100) / 100;
  if (rounded === 0) return "0,00";
  return `${rounded > 0 ? "+" : "−"}${decimal.format(Math.abs(rounded))}`;
}

/** Mods as they're played, NoFail left out (every map has it): "HDHR"; "NM" for none. */
export function formatMods(mods: string | null): string {
  if (mods === null) return "?";
  const shown = (mods.match(/[A-Z0-9]{2}/g) ?? []).filter((m) => m !== "NF").join("");
  return shown || "NM";
}

export const accuracyUnknown = "Doğruluk osu!'nun maç kaydından okunur; henüz okunmadı.";

/** The viewer's team in these statistics, if they play. */
export function ownTeamId(index: StatsIndex, userId: string | undefined): string | null {
  if (!userId) return null;
  return index.player.get(userId)?.team_id ?? null;
}

/** Players' and teams' names, compared the Turkish way, for search. */
export function matchesQuery(query: string, ...names: (string | undefined)[]): boolean {
  const q = query.trim().toLocaleLowerCase("tr");
  if (!q) return true;
  return names.some((n) => n?.toLocaleLowerCase("tr").includes(q));
}
