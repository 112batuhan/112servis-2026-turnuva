import { useEffect, useRef, useState, type ReactNode } from "react";
import type { CanPredict } from "../../api/types";
import { outcomes, type Outcome } from "../../lib/pickem";
import "./PickemSlider.css";

const CAN_PREDICT_HINT: Record<CanPredict, string | undefined> = {
  yes: undefined,
  sign_in: "Tahmin için giriş yap",
  discord: "Tahmin için Discord hesabını bağla",
  banned: "Turnuvadan banlı olduğun için tahmin yapamazsın",
};

interface Props {
  bestOf: number;
  teams: [{ id: string; name: string }, { id: string; name: string }];
  /** What's picked now: the side and the loser's maps. */
  picked: { slot: 1 | 2; loser: number } | null;
  canPredict: CanPredict;
  saving: boolean;
  label: string;
  /** Shows the teams' names at the ends of the track. */
  named?: boolean;
  /** What stands for each team at its end of the track, in place of its plain name. */
  renderName?: (slot: 1 | 2) => ReactNode;
  onPick: (teamId: string, loserScore: number) => void;
  /** Called when the thumb is let go in the middle over a saved prediction. */
  onClear: () => void;
}

/**
 * The ballot as a tug of war: one track per match, team 1's side on the left and team 2's on the
 * right. The middle is no prediction; the further the thumb goes towards a side, the bigger that
 * side's win (5–0 at the left end, the closest results either side of the middle). The thumb
 * carries the score, and letting go saves it. The stops' labels under the track are buttons too,
 * and the track works from the keyboard.
 *
 * The rail takes the pointer itself (captured, so a drag keeps going off the track) rather than
 * leaving it to the range input: phones don't reliably send a range input's pointerup, so a drag
 * there would move the thumb without saving. A drag the browser takes over for scrolling
 * (pointercancel) puts the thumb back where it was saved. The range input stays, out of the
 * pointer's way, for the keyboard and screen readers.
 */
export function PickemSlider({
  bestOf,
  teams,
  picked,
  canPredict,
  saving,
  label,
  named,
  renderName,
  onPick,
  onClear,
}: Props) {
  // Stops left to right: team 1's outcomes, the middle, then team 2's.
  const axis = outcomes(bestOf);
  const half = axis.length / 2;
  const stops: (Outcome | null)[] = [...axis.slice(0, half), null, ...axis.slice(half)];
  const max = stops.length - 1;
  const savedIndex = picked
    ? stops.findIndex((o) => o !== null && o.slot === picked.slot && o.loser === picked.loser)
    : half;

  const [value, setValue] = useState(savedIndex);
  // The pointer dragging the thumb, while one is.
  const dragging = useRef<number | null>(null);
  // A new saved value (or a failed save rolling back) moves the thumb, unless it's in a hand.
  useEffect(() => {
    if (dragging.current === null) setValue(savedIndex);
  }, [savedIndex]);

  const disabled = canPredict !== "yes" || saving;
  const commit = (index: number) => {
    if (index === savedIndex) return;
    const o = stops[index];
    if (o === null) {
      if (picked) onClear();
      return;
    }
    onPick(teams[o.slot - 1].id, o.loser);
  };

  // Arrow keys step through the stops; the last step saves once the keys rest.
  const keyTimer = useRef<number | undefined>(undefined);
  useEffect(() => () => window.clearTimeout(keyTimer.current), []);

  // The stop nearest a point across the track (which is inset by half a thumb at both ends, so its
  // edges are the end stops).
  const trackRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const stopAt = (clientX: number) => {
    const rect = trackRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0) return value;
    const t = (clientX - rect.left) / rect.width;
    return Math.min(max, Math.max(0, Math.round(t * max)));
  };

  const current = stops[value];
  const pct = (index: number) => (index / max) * 100;
  const side = current ? (current.slot === 1 ? "blue" : "red") : null;
  const valueText = current
    ? `${teams[current.slot - 1].name} ${current.winner}–${current.loser} kazanır`
    : "Tahmin yok";

  return (
    <div
      className={`pickem-slider${side ? ` is-${side}` : " is-none"}${saving ? " is-saving" : ""}${canPredict !== "yes" ? " is-blocked" : ""}`}
      title={CAN_PREDICT_HINT[canPredict]}
      style={{ ["--at" as string]: `${pct(value)}%` }}
    >
      {named && (
        // Plain names repeat the input's label, so screen readers skip them; ones that open a team's
        // card are controls in their own right.
        <div className="pickem-slider-names" aria-hidden={renderName ? undefined : true}>
          <span className="is-slot-1">{renderName ? renderName(1) : teams[0].name}</span>
          <span className="is-slot-2">{renderName ? renderName(2) : teams[1].name}</span>
        </div>
      )}
      <div
        className="pickem-slider-rail"
        onPointerDown={(e) => {
          if (disabled || (e.pointerType === "mouse" && e.button !== 0)) return;
          e.currentTarget.setPointerCapture(e.pointerId);
          dragging.current = e.pointerId;
          // So the arrow keys carry on from here; a range input brings up no keyboard.
          inputRef.current?.focus({ preventScroll: true });
          setValue(stopAt(e.clientX));
        }}
        onPointerMove={(e) => {
          if (dragging.current === e.pointerId) setValue(stopAt(e.clientX));
        }}
        onPointerUp={(e) => {
          if (dragging.current !== e.pointerId) return;
          dragging.current = null;
          const index = stopAt(e.clientX);
          setValue(index);
          commit(index);
        }}
        onPointerCancel={(e) => {
          if (dragging.current !== e.pointerId) return;
          dragging.current = null;
          setValue(savedIndex);
        }}
      >
        <input
          ref={inputRef}
          type="range"
          tabIndex={canPredict !== "yes" ? -1 : 0}
          className="pickem-slider-input"
          min={0}
          max={max}
          step={1}
          value={value}
          disabled={canPredict !== "yes"}
          aria-label={label}
          aria-valuetext={valueText}
          onChange={(e) => {
            // Only the keyboard (or a screen reader) moves it: the pointer never reaches it.
            if (saving) return;
            const index = Number(e.target.value);
            setValue(index);
            window.clearTimeout(keyTimer.current);
            keyTimer.current = window.setTimeout(() => commit(index), 450);
          }}
        />
        <div ref={trackRef} className="pickem-slider-track" aria-hidden="true">
          <span className="pickem-slider-half is-slot-1" />
          <span className="pickem-slider-half is-slot-2" />
          {/* The pull from the middle to the thumb, in the winning side's colour. */}
          <span className="pickem-slider-pull" />
          {stops.map((o, i) => (
            <span
              key={i}
              className={o === null ? "pickem-slider-stop is-middle" : "pickem-slider-stop"}
              style={{ left: `${pct(i)}%` }}
            />
          ))}
          <span className="pickem-slider-thumb">{current ? `${current.team1}–${current.team2}` : "?"}</span>
        </div>
      </div>
      <div className="pickem-slider-labels">
        {stops.map((o, i) =>
          o === null ? (
            <span key={i} className="pickem-slider-label is-middle" style={{ left: `${pct(i)}%` }} aria-hidden="true">
              ·
            </span>
          ) : (
            <button
              key={i}
              type="button"
              tabIndex={-1}
              className={`pickem-slider-label is-slot-${o.slot}${i === value ? " is-current" : ""}`}
              style={{ left: `${pct(i)}%` }}
              disabled={disabled}
              aria-label={`${teams[o.slot - 1].name} ${o.winner}–${o.loser} kazanır`}
              onClick={() => {
                setValue(i);
                commit(i);
              }}
            >
              {o.team1}–{o.team2}
            </button>
          ),
        )}
      </div>
    </div>
  );
}
