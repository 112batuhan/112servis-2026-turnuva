import { applyMods, formatLength } from "../utils/mods.js";

// A single beatmap card, shared by the map pool editor and the public pool page.
// Interactive bits are opt-in: pass `drag` to make it draggable, `onRemove` to show
// a remove button. With neither, it renders as a static, read-only card.
export default function MapCard({ bm, modifier, drag, onRemove }) {
  const s = modifier ? applyMods(bm, modifier) : bm;
  const changed = (a, b) => modifier && Math.abs(a - b) > 0.05;
  const statClass = (a, b) => (changed(a, b) ? "stat stat-mod" : "stat");

  return (
    <div
      className="map-card"
      draggable={Boolean(drag)}
      onDragStart={drag ? (e) => e.dataTransfer.setData("text/plain", JSON.stringify(drag)) : undefined}
    >
      {bm.cover_url && <img className="map-cover" src={bm.cover_url} alt="" />}
      <div className="map-body">
        <a
          className="map-title"
          href={`https://osu.ppy.sh/beatmaps/${bm.beatmap_id}`}
          target="_blank"
          rel="noreferrer"
        >
          {bm.artist} — {bm.title}
        </a>
        <div className="map-sub">
          [{bm.version}]{bm.creator ? ` · ${bm.creator}` : ""}
        </div>
        <div className="map-stats">
          <span className="stat">★{bm.star_rating.toFixed(2)}</span>
          <span className={statClass(s.bpm, bm.bpm)}>{Math.round(s.bpm)} bpm</span>
          <span className={statClass(s.total_length, bm.total_length)}>{formatLength(s.total_length)}</span>
          <span className={statClass(s.cs, bm.cs)}>CS {s.cs.toFixed(1)}</span>
          <span className={statClass(s.ar, bm.ar)}>AR {s.ar.toFixed(1)}</span>
          <span className={statClass(s.od, bm.od)}>OD {s.od.toFixed(1)}</span>
          <span className={statClass(s.hp, bm.hp)}>HP {s.hp.toFixed(1)}</span>
        </div>
      </div>
      {onRemove && (
        <button className="map-remove" onClick={onRemove} aria-label="remove map">
          ×
        </button>
      )}
    </div>
  );
}
