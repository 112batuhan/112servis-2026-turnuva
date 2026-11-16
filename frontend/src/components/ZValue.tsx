import { formatPoints } from "../lib/format";
import "./ZValue.css";

/**
 * Points (a Z-percentile: the share of the map's field a play beats, between 0 and 1) as their
 * figure beside a small meter that fills from its start, with the field's mean marked at its
 * middle. With `max` it's a sum of them over that many maps (the qualifier's seeding points),
 * filled towards `max`; it stays empty below 0.
 */
export function ZValue({
  value,
  max = 1,
  missing = false,
  size = "md",
}: {
  value: number;
  max?: number;
  missing?: boolean;
  size?: "md" | "lg";
}) {
  const share = max > 0 ? Math.max(0, Math.min(1, value / max)) : 0;
  const tone = missing ? "is-missing" : share >= 0.5 ? "is-up" : "is-down";
  return (
    <span className={`zval ${tone} is-${size}`}>
      <span className="zval-num">{formatPoints(value)}</span>
      <span className="zval-bar" aria-hidden="true">
        <span className="zval-fill" style={{ width: `${share * 100}%` }} />
      </span>
    </span>
  );
}
