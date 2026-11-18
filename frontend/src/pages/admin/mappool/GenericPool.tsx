import { useState, type FormEvent } from "react";
import type { PoolMap } from "../../../api/types";
import { Icon } from "../../../components/Icon";
import { MapCard, type MapCardEditor } from "../../../components/MapCard";
import { PoolSection } from "../../../components/PoolSection";
import { showsHrStats } from "../../../lib/category";
import { useMapDropZone } from "../../../lib/dragAndDrop";

export const GENERIC_POOL_ID = "generic-pool";

// What a map is added with: its mods, or FM / TB for a freemod or tiebreaker slot (played with
// Freemod, stats without mods; an FM map's card adds them under HR).
const MOD_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "NoMod" },
  { value: "HD", label: "HD" },
  { value: "HR", label: "HR" },
  { value: "DT", label: "DT" },
  { value: "HT", label: "HT" },
  { value: "EZ", label: "EZ" },
  { value: "FL", label: "FL" },
  { value: "HDHR", label: "HDHR" },
  { value: "HDDT", label: "HDDT" },
  { value: "FM", label: "FM · Freemod" },
  { value: "TB", label: "TB · Tiebreaker" },
];

interface Props {
  maps: PoolMap[];
  expanded: boolean;
  onToggle: () => void;
  busy: boolean;
  onAddMap: (beatmapId: number, mods: string) => Promise<boolean>;
  onDropMap: (mapId: string) => void;
  /** Absent for those who may only look: no adding, dropping or editing. */
  editorFor?: (map: PoolMap) => MapCardEditor;
}

/** Maps not placed in any slot. Shared by every stage. */
export function GenericPool({
  maps,
  expanded,
  onToggle,
  busy,
  onAddMap,
  onDropMap,
  editorFor,
}: Props) {
  const { isOver, dropProps } = useMapDropZone(onDropMap);

  return (
    <PoolSection
      title="Ortak havuz"
      meta={`${maps.length} map · tüm stage'lerde ortak`}
      expanded={expanded}
      onToggle={onToggle}
      isDropTarget={isOver}
      {...(editorFor ? dropProps : {})}
    >
      {editorFor && <AddMapForm busy={busy} onAddMap={onAddMap} />}
      {maps.length === 0 && (
        <p className="pool-empty">
          {editorFor
            ? "Yerleştirilmemiş map yok. Yukarıdan beatmap id'siyle map ekleyebilirsin."
            : "Yerleştirilmemiş map yok."}
        </p>
      )}
      {maps.map((map) => (
        <MapCard
          key={map.id}
          map={map}
          editor={editorFor?.(map)}
          viewOnly={!editorFor}
          showHr={showsHrStats(map)}
        />
      ))}
    </PoolSection>
  );
}

function AddMapForm({ busy, onAddMap }: Pick<Props, "busy" | "onAddMap">) {
  const [beatmapId, setBeatmapId] = useState("");
  const [mods, setMods] = useState("");
  const parsedId = parseInt(beatmapId, 10);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (parsedId > 0 && (await onAddMap(parsedId, mods))) setBeatmapId("");
  }

  return (
    <form className="inline-form add-map-form" onSubmit={submit}>
      <label className="sr-only" htmlFor="add-map-id">
        Beatmap id
      </label>
      <input
        id="add-map-id"
        className="input"
        value={beatmapId}
        onChange={(e) => setBeatmapId(e.target.value)}
        placeholder="Beatmap id…"
        inputMode="numeric"
        autoComplete="off"
      />
      <label className="sr-only" htmlFor="add-map-mods">
        Mods
      </label>
      <select
        id="add-map-mods"
        className="select"
        value={mods}
        onChange={(e) => setMods(e.target.value)}
      >
        {MOD_OPTIONS.map((m) => (
          <option key={m.value} value={m.value}>
            {m.label}
          </option>
        ))}
      </select>
      <button type="submit" className="btn btn-primary" disabled={busy || !(parsedId > 0)}>
        <Icon name="plus" />
        {busy ? "Ekleniyor…" : "Map ekle"}
      </button>
    </form>
  );
}
