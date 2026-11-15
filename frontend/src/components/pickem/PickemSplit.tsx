import type { PickemScoreCount } from "../../api/types";
import { countsByOutcome, outcomeKey, outcomes, shares } from "../../lib/pickem";
import "./PickemSplit.css";

interface SplitProps {
  team1: number;
  team2: number;
  /** The teams' names, said with the shares (screen readers, and over the bar when `named`). */
  names: [string, string];
  /** Shows the names and counts over the bar. */
  named?: boolean;
  /** The side the viewer picked. */
  mine?: 1 | 2 | null;
  /** Says how many predicted under the bar. */
  counted?: boolean;
}

/**
 * How a match's winner was predicted: one bar split in the room's sides (team 1 blue on the left,
 * team 2 red on the right), each share at its end, the bigger one in ink.
 */
export function PickemSplitBar({
  team1,
  team2,
  names,
  named = false,
  mine = null,
  counted = false,
}: SplitProps) {
  const total = team1 + team2;
  const [p1, p2] = shares(team1, team2);
  if (total === 0) return <p className="pickem-split-empty">Kimse tahmin etmedi.</p>;
  const lead = team1 === team2 ? null : team1 > team2 ? 1 : 2;
  const share = (slot: 1 | 2, pct: number, count: number) => (
    <span className={`pickem-split-share is-slot-${slot}${lead === slot ? " is-lead" : ""}`}>
      <span className="pickem-split-pct">
        <span className="pickem-split-unit">%</span>
        {pct}
      </span>
      {named && <span className="pickem-split-count">{count} kişi</span>}
      {mine === slot && <span className="pickem-split-mine">sen</span>}
    </span>
  );
  return (
    <div className={named ? "pickem-split is-named" : "pickem-split"}>
      {named && (
        <div className="pickem-split-names" aria-hidden="true">
          <span className="is-slot-1">{names[0]}</span>
          <span className="is-slot-2">{names[1]}</span>
        </div>
      )}
      <div
        className="pickem-split-line"
        role="img"
        aria-label={`${total} tahmin: %${p1} ${names[0]}, %${p2} ${names[1]}${mine ? `; sen ${names[mine - 1]} dedin` : ""}`}
      >
        {share(1, p1, team1)}
        <span className="pickem-split-track" aria-hidden="true">
          {team1 > 0 && (
            <span className="pickem-split-fill is-team-blue" style={{ flexGrow: team1 }} />
          )}
          {team2 > 0 && (
            <span className="pickem-split-fill is-team-red" style={{ flexGrow: team2 }} />
          )}
        </span>
        {share(2, p2, team2)}
      </div>
      {counted && (
        <span className="pickem-split-total">
          <span className="num">{total}</span> tahmin
        </span>
      )}
    </div>
  );
}

interface ChartProps {
  bestOf: number;
  scores: PickemScoreCount[];
  names: [string, string];
  /** The result as the sides read it, once the match is over (not a forfeit). */
  result?: [number, number] | null;
  /** The score the viewer predicted, as the sides read it. */
  mine?: [number, number] | null;
}

/**
 * Every score the match could end on, team 1's biggest win on the left through the closest results
 * to team 2's biggest on the right, each column as tall as its share of the predictions: the same
 * axis as the ballot's score chips.
 */
export function PickemScoreChart({ bestOf, scores, names, result, mine }: ChartProps) {
  const counts = countsByOutcome(scores);
  const total = scores.reduce((sum, s) => sum + s.count, 0);
  if (total === 0) return null;
  const axis = outcomes(bestOf);
  const max = Math.max(1, ...axis.map((o) => counts.get(outcomeKey(o)) ?? 0));
  const same = (pair: [number, number] | null | undefined, o: { team1: number; team2: number }) =>
    pair != null && pair[0] === o.team1 && pair[1] === o.team2;

  return (
    <figure className="pickem-chart">
      <ol className="pickem-chart-columns" style={{ ["--columns" as string]: axis.length }}>
        {axis.map((o) => {
          const count = counts.get(outcomeKey(o)) ?? 0;
          const pct = Math.round((count / total) * 100);
          const isResult = same(result, o);
          const isMine = same(mine, o);
          const label = `${o.team1}–${o.team2}`;
          return (
            <li
              key={outcomeKey(o)}
              className={`pickem-chart-col is-slot-${o.slot}${isResult ? " is-result" : ""}${count === max ? " is-mode" : ""}`}
              tabIndex={0}
              data-tip={`${label}: ${count} kişi · %${pct}`}
              aria-label={`${label}, ${names[o.slot - 1]} kazanır: ${count} kişi, yüzde ${pct}${isResult ? ", maçın sonucu" : ""}${isMine ? ", senin tahminin" : ""}`}
            >
              <span className="pickem-chart-plot" aria-hidden="true">
                <span
                  className={`pickem-chart-bar is-team-${o.slot === 1 ? "blue" : "red"}`}
                  style={{ height: `${(count / max) * 100}%` }}
                >
                  {count > 0 && (count === max || isResult || isMine) && (
                    <span className="pickem-chart-value">%{pct}</span>
                  )}
                </span>
              </span>
              <span className="pickem-chart-label" aria-hidden="true">
                {label}
              </span>
              <span className="pickem-chart-tags" aria-hidden="true">
                {isResult && <span className="pickem-chart-tag is-result">sonuç</span>}
                {isMine && <span className="pickem-chart-tag is-mine">sen</span>}
              </span>
            </li>
          );
        })}
      </ol>
      <figcaption className="pickem-chart-legend">
        <span className="is-slot-1">
          <span className="pickem-chart-swatch is-team-blue" aria-hidden="true" />
          {names[0]} kazanır
        </span>
        <span className="is-slot-2">
          {names[1]} kazanır
          <span className="pickem-chart-swatch is-team-red" aria-hidden="true" />
        </span>
      </figcaption>
    </figure>
  );
}
