import type { ReactNode } from "react";
import type { Stage } from "../api/types";
import { bestOfLabel, firstTo } from "../lib/bestOf";
import "./StageTabs.css";

interface Props {
  stages: Stage[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  /** Show a published/draft marker on each tab (editor only). */
  showStatus?: boolean;
  /** Extra controls after the tabs (editor buttons). */
  children?: ReactNode;
}

export function StageTabs({ stages, selectedId, onSelect, showStatus, children }: Props) {
  return (
    <div className="stage-bar">
      <nav className="stage-tabs" aria-label="Stage'ler">
        {stages.map((stage) => (
          <button
            key={stage.id}
            type="button"
            className="stage-tab"
            aria-current={stage.id === selectedId ? "page" : undefined}
            onClick={() => onSelect(stage.id)}
          >
            {stage.name}
            {stage.best_of !== null && !stage.is_qualifier && (
              <span className="stage-tab-bo" title={`${firstTo(stage.best_of)} map alan kazanır`}>
                {bestOfLabel(stage.best_of)}
              </span>
            )}
            {showStatus && <StageStatusDot published={stage.published} />}
          </button>
        ))}
      </nav>
      {children}
    </div>
  );
}

function StageStatusDot({ published }: { published: boolean }) {
  const label = published ? "Yayında" : "Taslak, herkese açık değil";
  return (
    <span className={`stage-dot ${published ? "is-published" : "is-draft"}`} title={label}>
      <span className="sr-only">({label})</span>
    </span>
  );
}
