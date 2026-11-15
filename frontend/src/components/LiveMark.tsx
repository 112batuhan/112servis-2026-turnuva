import "./LiveMark.css";

/**
 * "Canlı" with the glowing frost dot: a match being played right now. `compact` is the dot and a
 * smaller word, for bracket cards and calendar chips; `large` is the match page's headline mark.
 */
export function LiveMark({
  compact = false,
  large = false,
}: {
  compact?: boolean;
  large?: boolean;
}) {
  return (
    <span className={`live-mark${compact ? " is-compact" : ""}${large ? " is-large" : ""}`}>
      Canlı
    </span>
  );
}
