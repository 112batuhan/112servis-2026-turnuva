import { useState } from "react";
import EditableNote from "./EditableNote.jsx";

// A single beatmap card, shared by the map pool editor and the public pool page.
// Stats are firm (mod-adjusted, locked at add time) so the card just displays them.
// Notes live inside the card, to the right of the map info (see EditableNote):
//   - editor: pass `onSaveNote(field, value)` for editable public + hidden notes.
//   - public: no `onSaveNote` — the public note is shown read-only (editor note absent).
// Editing a note suspends the card's own dragging so the field stays usable.
// Interactive bits are opt-in: `drag` makes the card draggable, `onRemove` shows an ×.
export default function MapCard({ bm, drag, onRemove, onSaveNote }) {
  const [dragOn, setDragOn] = useState(true);
  return (
    <div
      className="map-card"
      draggable={Boolean(drag) && dragOn}
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

      {onSaveNote ? (
        <div className="map-notes">
          <label className="map-note-field">
            <span className="map-note-label">Public</span>
            <EditableNote
              value={bm.public_notes}
              placeholder="Public note…"
              onEditingChange={(on) => setDragOn(!on)}
              onSave={(v) => onSaveNote("public_notes", v)}
            />
          </label>
          <label className="map-note-field">
            <span className="map-note-label map-note-label-hidden">Private</span>
            <EditableNote
              value={bm.editor_notes}
              placeholder="Editor note (hidden)…"
              hidden
              onEditingChange={(on) => setDragOn(!on)}
              onSave={(v) => onSaveNote("editor_notes", v)}
            />
          </label>
        </div>
      ) : (
        bm.public_notes && (
          <div className="map-notes">
            <div className="map-note-field">
              <EditableNote value={bm.public_notes} />
            </div>
          </div>
        )
      )}

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
