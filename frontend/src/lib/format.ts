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
