/** 95 -> "1:35" */
export function formatLength(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

/** 12345 -> "#12,345", null -> "—" */
export function formatRank(rank: number | null | undefined): string {
  return rank ? `#${rank.toLocaleString()}` : "—";
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString();
}

export function plural(count: number, word: string): string {
  return `${count} ${word}${count === 1 ? "" : "s"}`;
}

/** 18876.6 -> "18,877" */
export function formatTeamValue(value: number | null | undefined): string {
  return value == null ? "—" : Math.round(value).toLocaleString();
}

/** Mirrors TEAM_NAME_MAX in backend/src/handlers/team.rs. */
export const TEAM_NAME_MAX = 32;

/** Mirrors BANNER_MAX_BYTES in backend/src/handlers/team.rs. */
export const BANNER_MAX_BYTES = 2 * 1024 * 1024;
