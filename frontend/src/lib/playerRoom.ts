import type { MapBan, MapPick } from "../api/types";
import type { StepKind } from "./draftOrder";
import type { PoolEntry } from "./matchPool";

// The rules the room page shows before a player asks: which maps their team may ban or pick, the
// warmup link they pasted, and the clocks. The room's bot decides (backend/src/bancho/autoref:
// draft.rs `refusal`, players.rs); this mirrors it so the page never offers what it would refuse.

/** How long a ban or pick is held on the room page before it's sent. */
export const HOLD_MS = 3000;
/** The bot acts this long after the end it told the room (autoref/countdown.rs GRACE). */
export const AUTOREF_GRACE_MS = 10_000;
/** A map can be aborted this long after it starts (autoref ABORT_WINDOW). */
export const ABORT_WINDOW_MS = 30_000;
/** A report's longest text (autoref REPORT_MAX). */
export const REPORT_MAX = 300;

/** Why the map can't be the team's ban or pick, in the page's words; null when it can. */
export function draftRefusal(
  entry: PoolEntry,
  kind: StepKind,
  teamId: string,
  draft: { bans: MapBan[]; picks: MapPick[] },
  played: Set<string | undefined>,
  entries: PoolEntry[],
): string | null {
  const id = entry.map.id;
  if (entry.tiebreaker) return kind === "pick" ? "Tiebreaker seçilemez" : "Tiebreaker banlanamaz";
  if (draft.bans.some((b) => b.map_id === id)) return "Banlandı";
  if (draft.picks.some((p) => p.map_id === id) || played.has(id)) return "Oynandı";
  if (kind === "pick" && !entry.repeatable) {
    const last = draft.picks
      .filter((p) => p.team_id === teamId)
      .reduce<MapPick | null>((a, p) => (!a || p.created_at > a.created_at ? p : a), null);
    const lastEntry = last ? entries.find((e) => e.map.id === last.map_id) : undefined;
    if (lastEntry && lastEntry.abbreviation === entry.abbreviation) {
      return `Son pick'in ${entry.abbreviation}, aynı mod arka arkaya seçilemez`;
    }
  }
  return null;
}

/**
 * The beatmap id in what a player pasted: a bare id, or an osu! link to a beatmap
 * (`/b/123`, `/beatmaps/123`, `/beatmapsets/1#osu/123`). A set's link without a difficulty names no
 * beatmap.
 */
export function parseBeatmapId(text: string): number | null {
  const t = text.trim();
  if (/^\d{1,10}$/.test(t)) return Number(t);
  const patterns = [
    /osu\.ppy\.sh\/beatmapsets\/\d+#[a-z]+\/(\d+)/i,
    /osu\.ppy\.sh\/beatmaps\/(\d+)/i,
    /osu\.ppy\.sh\/b\/(\d+)/i,
  ];
  for (const re of patterns) {
    const m = re.exec(t);
    if (m) return Number(m[1]);
  }
  return null;
}

/**
 * The end of the phase's clock as the room was told it: the bot's deadline less its grace, except
 * the match's own time before it comes, which has none.
 */
export function shownEnd(deadline: string | null, startsAt: string | null): number | null {
  if (!deadline) return null;
  const at = new Date(deadline).getTime();
  if (startsAt && new Date(startsAt).getTime() === at) return at;
  return at - AUTOREF_GRACE_MS;
}

/** "2:05", "0:09"; never below zero. */
export function clock(ms: number): string {
  const total = Math.max(0, Math.ceil(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}
