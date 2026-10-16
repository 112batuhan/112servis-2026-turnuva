import { useLayoutEffect, useRef, useState, type ReactNode } from "react";
import type { MapNoteField, PoolMap, PublicPoolMap } from "../api/types";
import { mapDragProps } from "../lib/dragAndDrop";
import { formatLength } from "../lib/format";
import { usePlayer } from "../player/PlayerContext";
import { EditableNote } from "./EditableNote";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";
import "./MapCard.css";

export interface MoveTargetGroup {
  category: string;
  slots: { id: string; label: string; occupied: boolean }[];
}

export interface MapCardEditor {
  onSaveNote: (field: MapNoteField, value: string) => void;
  /** null = back to the generic pool. */
  onMove: (slotId: string | null) => void;
  /** Permanent delete; only offered while the map is in the generic pool. */
  onDelete: () => void;
  moveTargets: MoveTargetGroup[];
}

interface Props {
  /** Public pages get maps without editor notes. */
  map: PublicPoolMap | PoolMap;
  /** Present only in the editor: enables dragging, notes and move/delete controls. */
  editor?: MapCardEditor;
  /** In the editor for those who may only look: shows both notes, without any controls. */
  viewOnly?: boolean;
}

export function MapCard({ map, editor, viewOnly }: Props) {
  const [noteOpen, setNoteOpen] = useState(false);
  const dragProps = editor && !noteOpen ? mapDragProps(map.id) : {};
  const editorNotes = "editor_notes" in map ? map.editor_notes : "";
  const showNotes = editor || (viewOnly && (map.public_notes || editorNotes));

  return (
    <article className={`map-card ${editor ? "is-editable" : ""}`} {...dragProps}>
      <div className="map-card-main">
        <PreviewButton map={map} />
        {map.cover_url && <Cover url={map.cover_url} />}
        <MapInfo map={map} note={editor || viewOnly ? undefined : map.public_notes} />
      </div>

      {showNotes && (
        <div className="map-card-notes">
          <EditableNote
            label="Herkese açık not"
            placeholder="Herkese açık not…"
            value={map.public_notes}
            readOnly={!editor}
            onEditingChange={setNoteOpen}
            onSave={(v) => editor?.onSaveNote("public_notes", v)}
          />
          <EditableNote
            label="Gizli editör notu"
            placeholder="Gizli not (sadece editörler)…"
            value={editorNotes}
            isPrivate
            readOnly={!editor}
            onEditingChange={setNoteOpen}
            onSave={(v) => editor?.onSaveNote("editor_notes", v)}
          />
        </div>
      )}

      <div className="map-card-actions">
        <CopyIdButton id={map.beatmap_id} />
        <a
          className="icon-btn"
          href={`https://beatconnect.io/b/${map.beatmapset_id}`}
          target="_blank"
          rel="noopener noreferrer"
          draggable={false}
          aria-label="Beatmap'i Beatconnect'ten indir"
          title="Beatmap'i indir (Beatconnect)"
        >
          <Icon name="download" />
        </a>
        {editor && <EditorActions map={map} editor={editor} />}
      </div>
    </article>
  );
}

/** `note` is the public note, shown at the end of the stat row on public pages. */
function MapInfo({ map, note }: { map: PublicPoolMap; note?: string }) {
  return (
    <div className="map-info">
      <div className="map-info-head">
        <a
          className="map-title"
          href={`https://osu.ppy.sh/beatmaps/${map.beatmap_id}`}
          target="_blank"
          rel="noopener noreferrer"
          draggable={false}
        >
          {map.artist} — {map.title}
        </a>
        <span className="map-meta">
          <span className="map-meta-label">Diff:</span> {map.version}
        </span>
        {map.creator && (
          <span className="map-meta">
            <span className="map-meta-label">Mapper:</span> {map.creator}
          </span>
        )}
      </div>
      <div className="map-stats-row">
        <dl className="map-stats">
          {map.mods && <Stat label="Mods" value={map.mods} highlight />}
          <Stat
            label="Star rating"
            value={
              <>
                <Icon name="star" size={12} className="map-stat-star" /> {map.star_rating.toFixed(2)}
              </>
            }
          />
          <Stat label="BPM" value={`${Math.round(map.bpm)} bpm`} />
          <Stat label="Uzunluk" value={formatLength(map.total_length)} />
          <Stat label="CS" value={`CS ${map.cs.toFixed(1)}`} />
          <Stat label="AR" value={`AR ${map.ar.toFixed(1)}`} />
          <Stat label="OD" value={`OD ${map.od.toFixed(1)}`} />
        </dl>
        {note && <PublicNote note={note} />}
      </div>
    </div>
  );
}

/** Centred when the stats leave too little room and it drops to a line of its own. CSS can't tell
 *  a wrapped flex item apart, so the line it sits on is measured whenever the row resizes. */
function PublicNote({ note }: { note: string }) {
  const ref = useRef<HTMLParagraphElement>(null);
  const [wrapped, setWrapped] = useState(false);

  useLayoutEffect(() => {
    const el = ref.current;
    const stats = el?.previousElementSibling;
    if (!el || !stats) return;
    const measure = () =>
      setWrapped(el.getBoundingClientRect().top >= stats.getBoundingClientRect().bottom);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    observer.observe(stats);
    return () => observer.disconnect();
  }, []);

  return (
    <p ref={ref} className={`map-card-public-note ${wrapped ? "is-wrapped" : ""}`}>
      {note}
    </p>
  );
}

function Stat({ label, value, highlight }: { label: string; value: ReactNode; highlight?: boolean }) {
  return (
    <div className={`map-stat ${highlight ? "is-highlight" : ""}`}>
      <dt className="sr-only">{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}

function PreviewButton({ map }: { map: PublicPoolMap }) {
  const { track, toggle } = usePlayer();
  const playing = track?.beatmapsetId === map.beatmapset_id;
  return (
    <IconButton
      icon={playing ? "stop" : "speaker"}
      label={playing ? "Önizlemeyi durdur" : "Önizlemeyi oynat"}
      variant="round"
      active={playing}
      onClick={() =>
        toggle({
          beatmapsetId: map.beatmapset_id,
          title: `${map.artist} — ${map.title}`,
          subtitle: `[${map.version}]`,
        })
      }
    />
  );
}

function Cover({ url }: { url: string }) {
  return (
    <div className="map-cover">
      <img className="map-cover-thumb" src={url} alt="" width={56} height={40} loading="lazy" />
      <img className="map-cover-zoom" src={url} alt="" loading="lazy" />
    </div>
  );
}

function CopyIdButton({ id }: { id: number }) {
  const [copied, setCopied] = useState(false);

  async function copy() {
    try {
      await navigator.clipboard.writeText(String(id));
      setCopied(true);
      setTimeout(() => setCopied(false), 1200);
    } catch {
      // clipboard unavailable (insecure context / permissions)
    }
  }

  return (
    <button
      type="button"
      className={`map-id ${copied ? "is-copied" : ""}`}
      onClick={copy}
      title="Beatmap id'sini kopyala"
      aria-label={`Beatmap id'sini kopyala: ${id}`}
    >
      <span aria-live="polite">{copied ? "Kopyalandı" : id}</span>
    </button>
  );
}

const GENERIC_POOL_OPTION = "__generic";

function EditorActions({ map, editor }: { map: PublicPoolMap; editor: MapCardEditor }) {
  const inSlot = map.slot_id !== null;

  return (
    <>
      <select
        className="select select-sm map-move"
        value=""
        aria-label="Map'i bir slot'a taşı"
        onChange={(e) => {
          const target = e.target.value;
          if (target) editor.onMove(target === GENERIC_POOL_OPTION ? null : target);
        }}
      >
        <option value="" disabled>
          Taşı…
        </option>
        {inSlot && <option value={GENERIC_POOL_OPTION}>Ortak havuz</option>}
        {editor.moveTargets.map((group) => (
          <optgroup key={group.category} label={group.category}>
            {group.slots.map((slot) => (
              <option key={slot.id} value={slot.id} disabled={slot.id === map.slot_id}>
                {slot.label}
                {slot.id === map.slot_id ? " (burada)" : slot.occupied ? " (yer değiştir)" : ""}
              </option>
            ))}
          </optgroup>
        ))}
      </select>
      {inSlot ? (
        <IconButton icon="undo" label="Ortak havuza geri gönder" onClick={() => editor.onMove(null)} />
      ) : (
        <IconButton icon="trash" label="Map'i sil" tone="danger" onClick={editor.onDelete} />
      )}
    </>
  );
}
