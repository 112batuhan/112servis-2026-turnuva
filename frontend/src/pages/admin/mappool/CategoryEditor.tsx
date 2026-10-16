import type { Category, PoolMap, Slot as SlotData } from "../../../api/types";
import { EditableNote } from "../../../components/EditableNote";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { MapCard, type MapCardEditor } from "../../../components/MapCard";
import { PoolSection } from "../../../components/PoolSection";
import { EmptySlot, Slot } from "../../../components/Slot";
import { useMapDropZone } from "../../../lib/dragAndDrop";
import type { StageEditorActions } from "./useStageEditor";

interface Props {
  category: Category;
  slots: SlotData[];
  /** Every map in the stage; each slot picks its own. */
  maps: PoolMap[];
  expanded: boolean;
  onToggle: () => void;
  busy: boolean;
  actions: StageEditorActions;
  /** Absent for those who may only look: no slots added or removed, nothing edited. */
  editorFor?: (map: PoolMap) => MapCardEditor;
}

export function CategoryEditor({ category, slots, maps, expanded, onToggle, busy, actions, editorFor }: Props) {
  return (
    <PoolSection
      title={category.name}
      meta={`${slots.length} slot`}
      color={category.color}
      expanded={expanded}
      onToggle={onToggle}
      actions={
        editorFor && (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            aria-label={`${category.name} kategorisine slot ekle`}
            disabled={busy}
            onClick={() => actions.addSlot(category.id)}
          >
            <Icon name="plus" size={14} />
            Slot
          </button>
        )
      }
    >
      {slots.length === 0 && (
        <p className="pool-empty">{editorFor ? "Map eklemek için slot ekle." : "Henüz slot yok."}</p>
      )}
      {slots.map((slot, i) => (
        <EditorSlot
          key={slot.id}
          slot={slot}
          categoryName={category.name}
          number={i + 1}
          map={maps.find((m) => m.slot_id === slot.id)}
          actions={actions}
          editorFor={editorFor}
        />
      ))}
    </PoolSection>
  );
}

interface EditorSlotProps {
  slot: SlotData;
  categoryName: string;
  number: number;
  map: PoolMap | undefined;
  actions: StageEditorActions;
  editorFor?: (map: PoolMap) => MapCardEditor;
}

function EditorSlot({ slot, categoryName, number, map, actions, editorFor }: EditorSlotProps) {
  const name = `${categoryName} #${number}`;
  const { isOver, dropProps } = useMapDropZone((mapId) => actions.moveMap(mapId, slot.id));

  function remove() {
    const hasContent = map || slot.editor_notes;
    if (hasContent && !confirm(`${name} slot'u kaldırılsın mı? Notları silinir ve map'i ortak havuza geri döner.`)) return;
    actions.deleteSlot(slot.id);
  }

  return (
    <Slot number={number} isDropTarget={isOver} {...(editorFor ? dropProps : {})}>
      {(editorFor || slot.editor_notes) && (
        <div className="slot-toolbar">
          <EditableNote
            label={`${name} gizli notu`}
            placeholder="Gizli slot notu (sadece editörler)…"
            value={slot.editor_notes}
            isPrivate
            readOnly={!editorFor}
            onSave={(v) => actions.saveSlotNotes(slot.id, v)}
          />
          {editorFor && <IconButton icon="close" label={`${name} slot'unu kaldır`} tone="danger" onClick={remove} />}
        </div>
      )}
      {map ? (
        <MapCard map={map} editor={editorFor?.(map)} viewOnly={!editorFor} />
      ) : (
        <EmptySlot>
          {editorFor ? `Buraya bir map sürükle ya da map üzerindeki "Taşı…" menüsünü kullan` : "Boş slot"}
        </EmptySlot>
      )}
    </Slot>
  );
}
