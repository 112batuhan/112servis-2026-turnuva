import { formatZ } from "../lib/format";
import "./ZValue.css";

// How many deviations fill half the meter.
const Z_SPAN = 3;

/**
 * A Z score as its figure beside a small meter centred on the map's mean: filled right of the
 * centre for a score above it, left for one below, in proportion up to three deviations.
 */
export function ZValue({
  z,
  missing = false,
  size = "md",
}: {
  z: number;
  missing?: boolean;
  size?: "md" | "lg";
}) {
  const half = (Math.min(Math.abs(z), Z_SPAN) / Z_SPAN) * 50;
  const tone = missing ? "is-missing" : z >= 0 ? "is-up" : "is-down";
  return (
    <span className={`zval ${tone} is-${size}`}>
      <span className="zval-num">{formatZ(z)}</span>
      <span className="zval-bar" aria-hidden="true">
        <span
          className="zval-fill"
          style={{ left: z >= 0 ? "50%" : `${50 - half}%`, width: `${half}%` }}
        />
      </span>
    </span>
  );
}
