import { useState } from "react";
import EditableNote from "./EditableNote.jsx";
import { usePlayer } from "../PlayerContext.jsx";
import "./MapCard.css";

// A single beatmap card, shared by the map pool editor and the public pool page.
// Stats are firm (mod-adjusted, locked at add time) so the card just displays them.
// Notes live inside the card, to the right of the map info (see EditableNote):
//   - editor: pass `onSaveNote(field, value)` for editable public + hidden notes.
//   - public: no `onSaveNote` — the public note is shown read-only (editor note absent).
// Editing a note suspends the card's own dragging so the field stays usable.
// Interactive bits are opt-in: `drag` makes the card draggable, `onRemove` shows an ×.
export default function MapCard({ bm, drag, onRemove, onSaveNote }) {
  const [dragOn, setDragOn] = useState(true);
  const { track, toggle } = usePlayer();
  const isPlaying = track?.id === bm.beatmapset_id;
  return (
    <div
      className="map-card"
      draggable={Boolean(drag) && dragOn}
      onDragStart={drag ? (e) => e.dataTransfer.setData("text/plain", JSON.stringify(drag)) : undefined}
    >
      <button
        className={`map-play ${isPlaying ? "is-playing" : ""}`}
        onClick={() =>
          toggle({
            id: bm.beatmapset_id,
            title: `${bm.artist} — ${bm.title}`,
            sub: `[${bm.version}]`,
          })
        }
        aria-label={isPlaying ? "Stop preview" : "Play preview"}
        title={isPlaying ? "Stop preview" : "Play preview"}
      >
        {isPlaying ? <StopGlyph /> : <SpeakerGlyph />}
      </button>
      {bm.cover_url && (
        <div className="map-cover-wrap">
          <img className="map-cover" src={bm.cover_url} alt="" />
          <img className="map-cover-zoom" src={bm.cover_url} alt="" loading="lazy" />
        </div>
      )}
      <div className="map-body">
        <div className="map-head">
          <a
            className="map-title"
            href={`https://osu.ppy.sh/beatmaps/${bm.beatmap_id}`}
            target="_blank"
            rel="noreferrer"
          >
            {bm.artist} — {bm.title}
          </a>
          <span className="map-diff">[{bm.version}]</span>
          {bm.creator && <span className="map-mapper">· {bm.creator}</span>}
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

      <CopyableId id={bm.beatmap_id} />
      <a
        className="map-dl"
        href={`https://beatconnect.io/b/${bm.beatmapset_id}`}
        target="_blank"
        rel="noreferrer"
        draggable={false}
        aria-label="Download beatmap"
        title="Download beatmap (Beatconnect)"
      >
        <DownloadGlyph />
      </a>
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

// The beatmap id, click to copy. Briefly swaps to a confirmation on success.
function CopyableId({ id }) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(String(id));
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      // clipboard blocked (insecure context / permissions) — leave the id as-is
    }
  };
  return (
    <button
      type="button"
      className={`map-id ${copied ? "is-copied" : ""}`}
      onClick={copy}
      title="Copy beatmap id"
      aria-label={`Copy beatmap id ${id}`}
    >
      {copied ? "Copied!" : id}
    </button>
  );
}

function SpeakerGlyph() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 9v6h4l5 4V5L8 9H4z" fill="currentColor" />
      <path
        d="M16 8.5a4.5 4.5 0 0 1 0 7M18.5 6a8 8 0 0 1 0 12"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.8"
        strokeLinecap="round"
      />
    </svg>
  );
}

function StopGlyph() {
  return (
    <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <rect x="5" y="5" width="14" height="14" rx="2" />
    </svg>
  );
}

function DownloadGlyph() {
  return (
    <svg
      width="15"
      height="15"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 3v12" />
      <path d="M7 11l5 5 5-5" />
      <path d="M5 21h14" />
    </svg>
  );
}
