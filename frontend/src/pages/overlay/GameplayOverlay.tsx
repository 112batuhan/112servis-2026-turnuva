import { useMemo, type CSSProperties } from "react";
import type { PublicMatch } from "../../api/types";
import { Icon } from "../../components/Icon";
import { formatLength } from "../../lib/format";
import type { PoolEntry } from "../admin/referee/pool";
import { matchTeams, poolEntries } from "../admin/referee/pool";
import type { LiveTotals } from "./useBroadcastMatch";
import {
  Crystal,
  MapPill,
  mapMarks,
  Marquee,
  OverlayStage,
  SIDE_CLASS,
  StageTitle,
  StarPips,
  TeamBanner,
  TrophyGlow,
  type Hole,
  type MapMark,
  type Side,
} from "./parts";
import type { BroadcastMatch } from "./useBroadcastMatch";
import "./GameplayOverlay.css";

// The overlay over the tournament client while a map is played (design/tournament-client-gameplay.png):
// the two halves of the client show through the holes, each team's banner, name and stars above
// its half, its bans and picks down its edge, the live score of the map under the client, and the
// map on in the room in the bottom left.

/** Where the tournament client's two halves show through, in frame pixels. */
const HOLES: Hole[] = [
  { x: 224, y: 80, w: 726, h: 816, r: [16, 0, 0, 16] },
  { x: 970, y: 80, w: 726, h: 816, r: [0, 16, 16, 0] },
];

export function GameplayOverlay({ view }: { view: BroadcastMatch | null }) {
  const match = view?.page.match ?? null;
  const tally = view?.page.tally ?? null;
  const team = (side: Side) => {
    const id = side === 1 ? match?.team1_id : match?.team2_id;
    return match?.teams.find((t) => t.id === id) ?? null;
  };
  const bestOf = match?.stage?.best_of ?? null;
  const marks = useMemo(() => (match ? mapMarks(match, tally) : new Map<string, MapMark>()), [match, tally]);
  const entries = useMemo(() => (match ? poolEntries(match) : []), [match]);
  const current = view?.page.currentEntry;

  return (
    <OverlayStage className="gp" holes={HOLES}>
      {([1, 2] as const).map((side) => (
        <div key={side} className={`gp-team ${SIDE_CLASS[side]}`}>
          <TeamBanner team={team(side)} side={side} />
          <h2 className="ov-team-name gp-team-name">{team(side)?.name ?? ""}</h2>
          <StarPips wins={(side === 1 ? tally?.team1 : tally?.team2) ?? 0} bestOf={bestOf} side={side} />
          <DraftColumn side={side} entries={entries} marks={marks} currentId={current?.map.id ?? null} />
        </div>
      ))}
      <StageTitle match={match} />
      <TrophyGlow className="gp-trophy" />
      <span className="gp-divider" aria-hidden="true" />
      <ScoreBar totals={view?.totals ?? null} />
      {match && current && <MapStrip key={current.map.id} match={match} entry={current} />}
    </OverlayStage>
  );
}

/** A team's bans, then its picks, in the order the referee marked them, down its edge of the frame. */
function DraftColumn({
  side,
  entries,
  marks,
  currentId,
}: {
  side: Side;
  entries: PoolEntry[];
  marks: Map<string, MapMark>;
  currentId: string | null;
}) {
  const rows = entries
    .flatMap((entry) => {
      const mark = marks.get(entry.map.id);
      return mark && mark.side === side ? [{ entry, mark }] : [];
    })
    .sort((a, b) => (a.mark.kind === b.mark.kind ? a.mark.at.localeCompare(b.mark.at) : a.mark.kind === "ban" ? -1 : 1));
  // Seven fit at the designed spacing; a longer match closes them up.
  const pitch = rows.length > 7 ? Math.floor(690 / rows.length) : 106;
  return (
    <ol className={`gp-draft ${SIDE_CLASS[side]}`} style={{ "--gp-pitch": `${pitch}px` } as CSSProperties}>
      {rows.map(({ entry, mark }) => (
        <li key={entry.map.id}>
          <MapPill entry={entry} mark={mark} tag={entry.label} current={entry.map.id === currentId} title={false} />
        </li>
      ))}
    </ol>
  );
}

const formatScore = (n: number) => n.toLocaleString("tr-TR");

/**
 * The map's live score under the client: each team's total at its end, a bar from each end
 * towards the middle in proportion to the leader's, and the lead in the middle in the leader's
 * colour. Without tosu it stays empty rather than showing a made-up score.
 */
function ScoreBar({ totals }: { totals: LiveTotals | null }) {
  const [a, b] = totals ?? [0, 0];
  const top = Math.max(a, b, 1);
  const lead = Math.abs(a - b);
  const leader: Side | null = a > b ? 1 : b > a ? 2 : null;
  return (
    <div className={`gp-score ${totals ? "" : "is-empty"}`} aria-live="off">
      <span className="gp-score-total is-side-1">{totals ? formatScore(a) : ""}</span>
      <span className="gp-score-bar is-side-1">
        <span style={{ width: `${(a / top) * 100}%` }} />
      </span>
      <span className={`gp-score-lead ${leader ? SIDE_CLASS[leader] : ""}`}>{lead > 0 ? `+${formatScore(lead)}` : ""}</span>
      <span className="gp-score-bar is-side-2">
        <span style={{ width: `${(b / top) * 100}%` }} />
      </span>
      <span className="gp-score-total is-side-2">{totals ? formatScore(b) : ""}</span>
    </div>
  );
}

/** Up to two decimals, without trailing zeros: 3.55, 10.3, 6. */
const stat = (n: number) => String(Math.round(n * 100) / 100);

/**
 * The map on in the room: its cover, title and artist on a frost-teal card, its stats on dark tabs
 * over the card's top edge, its star rating under the cover, the slot in large type at its end and
 * the team that picked it under the title.
 */
function MapStrip({ match, entry }: { match: PublicMatch; entry: PoolEntry }) {
  const m = entry.map;
  const pick = match.picks.find((p) => p.map_id === m.id);
  const picker = pick ? matchTeams(match).find((t) => t.id === pick.team_id) : undefined;
  const pickerSide: Side | null = picker ? (picker.id === match.team1_id ? 1 : 2) : null;
  return (
    <section className="gp-strip" aria-label="Oynanan map">
      <div className="gp-strip-card">
        <div className="gp-strip-cover">{m.cover_url && <img src={m.cover_url} alt="" />}</div>
        <div className="gp-strip-text">
          <Marquee start className="gp-strip-title" text={`${m.title} [${m.version}]`} />
          <Marquee start className="gp-strip-artist" text={m.artist} />
        </div>
        <span className="gp-strip-slot">{entry.label.toLocaleLowerCase("tr-TR")}</span>
      </div>
      <div className="gp-strip-tabs">
        <span className="gp-chip">
          <Icon name="metronome" size={18} />
          {Math.round(m.bpm)}
        </span>
        <span className="gp-chip">
          <Icon name="timer" size={18} />
          {formatLength(m.total_length)}
        </span>
        <span className="gp-chip is-stats">
          <span>cs:{stat(m.cs)}</span>
          <span>ar:{stat(m.ar)}</span>
          <span>od:{stat(m.od)}</span>
        </span>
      </div>
      <span className="gp-chip gp-strip-sr">
        <Crystal kind="won" className="gp-strip-sr-mark" />
        {m.star_rating.toFixed(2)}
      </span>
      {(picker || entry.tiebreaker) && (
        <span className={`gp-chip gp-strip-picker ${pickerSide ? SIDE_CLASS[pickerSide] : ""}`}>
          {picker ? picker.name : "Tiebreaker"}
        </span>
      )}
    </section>
  );
}
