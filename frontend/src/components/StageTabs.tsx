import type { ReactNode } from "react";
import type { Stage } from "../api/types";
import { useDragReorder } from "../lib/dragAndDrop";
import "./StageTabs.css";

interface Props {
  stages: Stage[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  /** Enables drag-to-reorder (editor only). */
  onDragMove?: (draggedId: string, targetId: string) => void;
  /** Show a published/draft marker on each tab (editor only). */
  showStatus?: boolean;
  /** Extra controls after the tabs (new-stage form, settings button). */
  children?: ReactNode;
}

export function StageTabs({ stages, selectedId, onSelect, onDragMove, showStatus, children }: Props) {
  const reorderProps = useDragReorder((dragged, target) => onDragMove?.(dragged, target));

  return (
    <div className="stage-bar">
      <nav className="stage-tabs" aria-label="Stages">
        {stages.map((stage) => (
          <button
            key={stage.id}
            type="button"
            className="stage-tab"
            aria-current={stage.id === selectedId ? "page" : undefined}
            onClick={() => onSelect(stage.id)}
            {...(onDragMove ? reorderProps(stage.id) : {})}
          >
            {stage.name}
            {showStatus && <StageStatusDot published={stage.published} />}
          </button>
        ))}
      </nav>
      {children}
    </div>
  );
}

function StageStatusDot({ published }: { published: boolean }) {
  const label = published ? "Published" : "Draft, not public";
  return (
    <span className={`stage-dot ${published ? "is-published" : "is-draft"}`} title={label}>
      <span className="sr-only">({label})</span>
    </span>
  );
}
