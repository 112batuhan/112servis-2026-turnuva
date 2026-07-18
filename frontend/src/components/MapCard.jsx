// A single beatmap card, shared by the map pool editor and the public pool page.
// Stats are firm — already mod-adjusted for placed entries, nomod for generic-pool
// maps — so the card just displays them (no client-side mod math). Interactive bits
// are opt-in: pass `drag` to make it draggable, `onRemove` to show a remove button.
export default function MapCard({ bm, drag, onRemove }) {
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
          {bm.mods && <span className="stat stat-mod">{bm.mods}</span>}
          <span className="stat">★{bm.star_rating.toFixed(2)}</span>
          <span className="stat">{Math.round(bm.bpm)} bpm</span>
          <span className="stat">{formatLength(bm.total_length)}</span>
          <span className="stat">CS {bm.cs.toFixed(1)}</span>
          <span className="stat">AR {bm.ar.toFixed(1)}</span>
          <span className="stat">OD {bm.od.toFixed(1)}</span>
          <span className="stat">HP {bm.hp.toFixed(1)}</span>
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

function formatLength(seconds) {
  const m = Math.floor(seconds / 60);
  const s = Math.round(seconds % 60);
  return `${m}:${s.toString().padStart(2, "0")}`;
}
