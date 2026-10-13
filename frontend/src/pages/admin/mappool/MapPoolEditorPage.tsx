import { useMemo, useState, type FormEvent } from "react";
import type { PoolMap, StageDetail } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { Icon } from "../../../components/Icon";
import type { MapCardEditor, MoveTargetGroup } from "../../../components/MapCard";
import { StageTabs } from "../../../components/StageTabs";
import { useExpandedSet } from "../../../lib/useExpandedSet";
import { CategoryEditor } from "./CategoryEditor";
import { GenericPool, GENERIC_POOL_ID } from "./GenericPool";
import { StageSettings } from "./StageSettings";
import { useStageEditor } from "./useStageEditor";
import "./MapPoolEditor.css";

export function MapPoolEditorPage() {
  const user = useSignedInUser();
  const { stages, selectedId, selectStage, detail, error, clearError, busy, actions } = useStageEditor(user.id);
  const [showSettings, setShowSettings] = useState(false);
  const { isExpanded, toggle } = useExpandedSet("mappool-expanded");
  const moveTargets = useMemo(() => buildMoveTargets(detail), [detail]);

  const editorFor = (map: PoolMap): MapCardEditor => ({
    moveTargets,
    onSaveNote: (field, value) => actions.saveMapNote(map.id, field, value),
    onMove: (slotId) => actions.moveMap(map.id, slotId),
    onDelete: () => {
      if (confirm(`Delete "${map.artist} - ${map.title} [${map.version}]" permanently?`)) actions.deleteMap(map.id);
    },
  });

  return (
    <div className="mappool-editor">
      <h1 className="sr-only">Map pool editor</h1>
      <ErrorAlert message={error} onDismiss={clearError} />

      <StageTabs
        stages={stages}
        selectedId={selectedId}
        onSelect={selectStage}
        onDragMove={actions.dragStage}
        showStatus
      >
        <NewStageForm busy={busy} onCreate={actions.createStage} />
        {detail && (
          <button
            type="button"
            className="btn btn-secondary"
            aria-pressed={showSettings}
            onClick={() => setShowSettings((v) => !v)}
          >
            <Icon name="settings" />
            Stage settings
          </button>
        )}
      </StageTabs>

      {!selectedId && <p className="status">Create a stage to start building a pool.</p>}

      {detail && (
        <>
          {showSettings && <StageSettings detail={detail} stages={stages} busy={busy} actions={actions} />}

          <div className="pool-board">
            <GenericPool
              maps={detail.maps.filter((m) => m.slot_id === null)}
              expanded={isExpanded(GENERIC_POOL_ID)}
              onToggle={() => toggle(GENERIC_POOL_ID)}
              busy={busy}
              onAddMap={actions.addMap}
              onDropMap={(mapId) => actions.moveMap(mapId, null)}
              editorFor={editorFor}
            />
            {detail.categories.length === 0 && (
              <p className="status">No categories yet. Add them from Stage settings.</p>
            )}
            {detail.categories.map((category) => (
              <CategoryEditor
                key={category.id}
                category={category}
                slots={detail.slots.filter((s) => s.category_id === category.id)}
                maps={detail.maps}
                expanded={isExpanded(category.id)}
                onToggle={() => toggle(category.id)}
                busy={busy}
                actions={actions}
                editorFor={editorFor}
              />
            ))}
          </div>
        </>
      )}
    </div>
  );
}

function NewStageForm({ busy, onCreate }: { busy: boolean; onCreate: (name: string) => Promise<boolean> }) {
  const [name, setName] = useState("");

  async function submit(e: FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed && (await onCreate(trimmed))) setName("");
  }

  return (
    <form className="inline-form new-stage-form" onSubmit={submit}>
      <label className="sr-only" htmlFor="new-stage-name">
        New stage name
      </label>
      <input
        id="new-stage-name"
        className="input"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="New stage…"
      />
      <button type="submit" className="btn btn-primary" disabled={busy || !name.trim()}>
        <Icon name="plus" />
        Add stage
      </button>
    </form>
  );
}

/** The "Move to…" options on every map card: each category's slots, in order. */
function buildMoveTargets(detail: StageDetail | null): MoveTargetGroup[] {
  if (!detail) return [];
  const occupied = new Set(detail.maps.map((m) => m.slot_id));
  return detail.categories.map((category) => ({
    category: category.name,
    slots: detail.slots
      .filter((s) => s.category_id === category.id)
      .map((slot, i) => ({ id: slot.id, label: `${category.name} #${i + 1}`, occupied: occupied.has(slot.id) })),
  }));
}
