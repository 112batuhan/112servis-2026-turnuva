import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import type { PoolMap, StageDetail } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
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
  const { stages, selectedId, selectStage, detail, error, clearError, busy, actions } =
    useStageEditor(user.id);
  const [showSettings, setShowSettings] = useState(false);
  const { isExpanded, toggle } = useExpandedSet("mappool-expanded");
  const moveTargets = useMemo(() => buildMoveTargets(detail), [detail]);
  // Pool testers and custom mappers only look.
  const canEdit = hasRole(user, "map_pooler");

  const editorFor = (map: PoolMap): MapCardEditor => ({
    moveTargets,
    onSaveNote: (field, value) => actions.saveMapNote(map.id, field, value),
    onMove: (slotId) => actions.moveMap(map.id, slotId),
    onRecompute: () => actions.recomputeMapStats(map.id),
    onDelete: () => {
      if (confirm(`"${map.artist} - ${map.title} [${map.version}]" kalıcı olarak silinsin mi?`))
        actions.deleteMap(map.id);
    },
  });

  return (
    <div className="mappool-editor">
      <h1 className="sr-only">Mappool editörü</h1>
      <ErrorAlert message={error} onDismiss={clearError} />

      <StageTabs stages={stages} selectedId={selectedId} onSelect={selectStage} showStatus>
        {hasRole(user, "host") && (
          <Link to="/admin/stages" className="btn btn-secondary">
            Stage'leri düzenle
          </Link>
        )}
        {detail && canEdit && (
          <button
            type="button"
            className="btn btn-secondary"
            aria-pressed={showSettings}
            onClick={() => setShowSettings((v) => !v)}
          >
            <Icon name="settings" />
            Stage ayarları
          </button>
        )}
        {!canEdit && <span className="badge">Sadece görüntüleme</span>}
      </StageTabs>

      {!selectedId && (
        <p className="status">
          {hasRole(user, "host") ? (
            <>
              Mappool oluşturmaya başlamak için <Link to="/admin/stages">Stage'ler</Link>{" "}
              sekmesinden bir stage ekle.
            </>
          ) : (
            "Henüz stage yok. Stage'leri hostlar ekler."
          )}
        </p>
      )}

      {detail && (
        <>
          {showSettings && canEdit && (
            <StageSettings detail={detail} busy={busy} actions={actions} />
          )}

          <div className="pool-board">
            <GenericPool
              maps={detail.maps.filter((m) => m.slot_id === null)}
              expanded={isExpanded(GENERIC_POOL_ID)}
              onToggle={() => toggle(GENERIC_POOL_ID)}
              busy={busy}
              onAddMap={actions.addMap}
              onDropMap={(mapId) => actions.moveMap(mapId, null)}
              editorFor={canEdit ? editorFor : undefined}
            />
            {detail.categories.length === 0 && (
              <p className="status">
                {canEdit
                  ? "Henüz kategori yok. Stage ayarlarından ekleyebilirsin."
                  : "Henüz kategori yok."}
              </p>
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
                editorFor={canEdit ? editorFor : undefined}
              />
            ))}
          </div>
        </>
      )}
    </div>
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
      .map((slot, i) => ({
        id: slot.id,
        label: `${category.name} #${i + 1}`,
        occupied: occupied.has(slot.id),
      })),
  }));
}
