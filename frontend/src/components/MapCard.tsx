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
  /** Resolves the map's stats from osu! again. */
  onRecompute: () => void;
  moveTargets: MoveTargetGroup[];
}

interface Props {
  /** Public pages get maps without editor notes. */
  map: PublicPoolMap | PoolMap;
  /** Present only in the editor: enables dragging, notes and move/delete controls. */
  editor?: MapCardEditor;
  /** In the editor for those who may only look: shows both notes, without any controls. */
  viewOnly?: boolean;
  /** An FM map, where players may take HR (not TB; lib/category.ts): adds the map's HR stats. */
  showHr?: boolean;
}

export function MapCard({ map, editor, viewOnly, showHr }: Props) {
  const [noteOpen, setNoteOpen] = useState(false);
  const dragProps = editor && !noteOpen ? mapDragProps(map.id) : {};
  const editorNotes = "editor_notes" in map ? map.editor_notes : "";
  const showNotes = editor || (viewOnly && (map.public_notes || editorNotes));

  return (
    <article className={`map-card ${editor ? "is-editable" : ""}`} {...dragProps}>
      <div className="map-card-main">
        <PreviewButton map={map} />
        {map.cover_url && <Cover url={map.cover_url} />}
        <MapInfo
          map={map}
          note={editor || viewOnly ? undefined : map.public_notes}
          hr={showHr ? (hrStatsOf(map) ?? (editor ? "missing" : null)) : null}
          onRecompute={editor?.onRecompute}
        />
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

interface HrStats {
  starRating: number;
  cs: number;
  ar: number;
  od: number;
}

function hrStatsOf(map: PublicPoolMap): HrStats | null {
  const { hr_star_rating, hr_cs, hr_ar, hr_od } = map;
  if (hr_star_rating === null || hr_cs === null || hr_ar === null || hr_od === null) return null;
  return { starRating: hr_star_rating, cs: hr_cs, ar: hr_ar, od: hr_od };
}

interface MapInfoProps {
  map: PublicPoolMap;
  /** The public note, shown at the end of the stat row on public pages. */
  note?: string;
  /** The HR stats under the nomod ones; "missing" in the editor for a map added before they
   *  were stored. */
  hr: HrStats | "missing" | null;
  onRecompute?: () => void;
}

function MapInfo({ map, note, hr, onRecompute }: MapInfoProps) {
  const stats = hr === "missing" ? null : hr;
  const hrValue = (render: (stats: HrStats) => ReactNode) => (stats ? render(stats) : undefined);

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
        <div className={`map-stats-table ${stats ? "has-hr" : ""}`}>
          {/* An FM map's mark leads its table, before the NM / HR heads, between the two lines. */}
          {stats && map.mods && (
            <p className="map-stats-mark">
              <span className="sr-only">Mods: </span>
              {map.mods}
            </p>
          )}
          {stats && (
            <div className="map-stats-modes" aria-hidden="true">
              <span>NM</span>
              <span>HR</span>
            </div>
          )}
          <dl className="map-stats">
            {!stats && map.mods && <Stat label="Mods" value={map.mods} highlight />}
            <Stat
              label="Star rating"
              value={<StarRating value={map.star_rating} />}
              hr={hrValue((s) => (
                <StarRating value={s.starRating} />
              ))}
            />
            <Stat label="BPM" value={`${Math.round(map.bpm)} bpm`} between={!!stats} />
            <Stat label="Uzunluk" value={formatLength(map.total_length)} between={!!stats} />
            <Stat
              label="CS"
              value={`CS ${map.cs.toFixed(1)}`}
              hr={hrValue((s) => `CS ${s.cs.toFixed(1)}`)}
            />
            <Stat
              label="AR"
              value={`AR ${map.ar.toFixed(1)}`}
              hr={hrValue((s) => `AR ${s.ar.toFixed(1)}`)}
            />
            <Stat
              label="OD"
              value={`OD ${map.od.toFixed(1)}`}
              hr={hrValue((s) => `OD ${s.od.toFixed(1)}`)}
            />
          </dl>
        </div>
        {hr === "missing" && (
          <p className="map-stats-missing">
            HR değerleri kayıtlı değil.{" "}
            {onRecompute && (
              <button type="button" className="map-stats-recompute" onClick={onRecompute}>
                Yeniden hesapla
              </button>
            )}
          </p>
        )}
        {note && <PublicNote note={note} />}
      </div>
    </div>
  );
}

function StarRating({ value }: { value: number }) {
  return (
    <>
      <Icon name="star" size={12} className="map-stat-star" /> {value.toFixed(2)}
    </>
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

interface StatProps {
  label: string;
  value: ReactNode;
  /** The value under HR, on the HR line below. */
  hr?: ReactNode;
  /** On a two-line table, a stat HR doesn't change: it sits between the lines, for both. */
  between?: boolean;
  highlight?: boolean;
}

function Stat({ label, value, hr, between, highlight }: StatProps) {
  return (
    <div className={`map-stat ${highlight ? "is-highlight" : ""} ${between ? "is-between" : ""}`}>
      <dt className="sr-only">{label}</dt>
      <dd>{value}</dd>
      {hr !== undefined && (
        <dd className="map-stat-hr">
          <span className="sr-only">HR ile: </span>
          {hr}
        </dd>
      )}
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
      <IconButton
        icon="refresh"
        label="İstatistikleri osu!'dan yeniden hesapla"
        onClick={editor.onRecompute}
      />
      {inSlot ? (
        <IconButton
          icon="undo"
          label="Ortak havuza geri gönder"
          onClick={() => editor.onMove(null)}
        />
      ) : (
        <IconButton icon="trash" label="Map'i sil" tone="danger" onClick={editor.onDelete} />
      )}
    </>
  );
}
