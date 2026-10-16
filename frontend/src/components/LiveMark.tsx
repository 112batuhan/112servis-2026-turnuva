import "./LiveMark.css";

/**
 * "Canlı" with the glowing frost dot: a match being played right now. `compact` is the dot and a
 * smaller word, for bracket cards and calendar chips.
 */
export function LiveMark({ compact = false }: { compact?: boolean }) {
  return <span className={compact ? "live-mark is-compact" : "live-mark"}>Canlı</span>;
}
