/** 95 -> "1:35" */
export function formatLength(seconds: number): string {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}:${String(s).padStart(2, "0")}`;
}

/** 12345 -> "#12.345", null -> "—" */
export function formatRank(rank: number | null | undefined): string {
  return rank ? `#${rank.toLocaleString("tr-TR")}` : "—";
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("tr-TR");
}

export function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString("tr-TR", { dateStyle: "medium", timeStyle: "short" });
}

/** "15 Ekim Perşembe 21:00" */
export function formatMatchTime(iso: string): string {
  return new Date(iso).toLocaleString("tr-TR", {
    weekday: "long",
    day: "numeric",
    month: "long",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** "15 Eki Per 21:00" */
export function formatShortMatchTime(iso: string): string {
  return new Date(iso).toLocaleString("tr-TR", {
    weekday: "short",
    day: "numeric",
    month: "short",
    hour: "2-digit",
    minute: "2-digit",
  });
}

/** "21:00", from an ISO time or a timestamp. */
export function formatClock(at: string | number): string {
  return new Date(at).toLocaleTimeString("tr-TR", { hour: "2-digit", minute: "2-digit" });
}

/** "21:00:07", for chat lines. */
export function formatClockSeconds(at: string | number): string {
  return new Date(at).toLocaleTimeString("tr-TR", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/** "15 Ekim Perşembe", or "15 Ekim 2026 Perşembe" with the year. */
export function formatDay(day: Date, withYear = false): string {
  return day.toLocaleDateString("tr-TR", {
    day: "numeric",
    month: "long",
    weekday: "long",
    ...(withYear ? { year: "numeric" } : {}),
  });
}

/** "Ekim 2026" */
export function formatMonth(month: Date): string {
  return month.toLocaleDateString("tr-TR", { month: "long", year: "numeric" });
}

/** 1234567 -> "1.234.567" */
export function formatScore(score: number): string {
  return score.toLocaleString("tr-TR");
}

/** ISO -> the local "YYYY-MM-DDTHH:mm" a datetime-local input takes. */
export function toDateTimeLocal(iso: string): string {
  const d = new Date(iso);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** A datetime-local input's value, read as local time -> ISO. */
export function fromDateTimeLocal(value: string): string {
  return new Date(value).toISOString();
}

/** 18876.6 -> "18.877" */
export function formatTeamValue(value: number | null | undefined): string {
  return value == null ? "—" : Math.round(value).toLocaleString("tr-TR");
}

/** Mirrors TEAM_NAME_MAX in backend/src/handlers/team.rs. */
export const TEAM_NAME_MAX = 32;

/** Mirrors MAX_UPLOAD_BYTES in backend/src/banner.rs. */
export const BANNER_MAX_BYTES = 2 * 1024 * 1024;

const twoDecimals = new Intl.NumberFormat("tr-TR", {
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

/** Points (a Z-percentile, or a sum of them): 0,84 / 7,25 / −1,00, with a real minus sign. */
export function formatPoints(points: number): string {
  const rounded = Math.round(points * 100) / 100;
  return `${rounded < 0 ? "−" : ""}${twoDecimals.format(Math.abs(rounded))}`;
}
