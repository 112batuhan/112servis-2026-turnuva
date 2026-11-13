import type { ReactNode } from "react";
import type { Stage } from "../api/types";
import { bestOfLabel, firstTo } from "../lib/bestOf";
import "./StageTabs.css";

interface Props {
  stages: Stage[];
  selectedId: string | null;
  onSelect: (id: string) => void;
  /** The stage the tournament is in (lib/stageTrack.ts): lit in frost with the crystal's glow, "Şu an" under it. */
  currentId?: string | null;
  /** Matches being played now, by stage (the stats' activity): said in frost under the stage's name. */
  live?: ReadonlyMap<string, number>;
  /** Say on each stage whether it's published or a draft (editor only). */
  showStatus?: boolean;
  /** Extra controls after the track (the pack download, editor buttons). */
  children?: ReactNode;
}

/**
 * The stages in play order as the site's stage track (the landing's, /stats', pick'em's): a 3px
 * rule over each stage's name and best of, the shown one on the hover wash in ink, the current
 * one lit. It wraps into a grid rather than scrolling. In the editor a draft's rule is dashed.
 */
export function StageTabs({ stages, selectedId, onSelect, currentId, live, showStatus, children }: Props) {
  return (
    <div className="stage-bar">
      <nav className="stage-tabs" aria-label="Stage'ler">
        <ol>
          {stages.map((stage) => {
            const current = stage.id === currentId;
            const classes = ["stage-tab", current && "is-current", showStatus && !stage.published && "is-draft"];
            return (
              <li key={stage.id}>
                <button
                  type="button"
                  className={classes.filter(Boolean).join(" ")}
                  aria-pressed={stage.id === selectedId}
                  onClick={() => onSelect(stage.id)}
                >
                  <span className="stage-tab-head">
                    <span className="stage-tab-name">{stage.name}</span>
                    {stage.best_of !== null && !stage.is_qualifier && (
                      <span className="stage-tab-bo" title={`${firstTo(stage.best_of)} map alan kazanır`}>
                        {bestOfLabel(stage.best_of)}
                      </span>
                    )}
                  </span>
                  <StageNote stage={stage} current={current} live={live?.get(stage.id) ?? 0} showStatus={showStatus} />
                </button>
              </li>
            );
          })}
        </ol>
      </nav>
      {children && <div className="stage-bar-actions">{children}</div>}
    </div>
  );
}

interface NoteProps {
  stage: Stage;
  current: boolean;
  live: number;
  showStatus?: boolean;
}

/** What's under the name: matches being played (else "Şu an" on the current stage), and in the editor whether it's published. */
function StageNote({ stage, current, live, showStatus }: NoteProps) {
  const now =
    live > 0 ? `${live} ${stage.is_qualifier ? "lobi" : "maç"} oynanıyor` : current ? "Şu an" : null;
  if (!now && !showStatus) return null;
  return (
    <span className="stage-tab-note">
      {now && <span className="stage-tab-now">{now}</span>}
      {showStatus && <StageStatus published={stage.published} />}
    </span>
  );
}

/** Published: a filled green dot; draft: a hollow amber ring, each with its word so it isn't colour-only. */
function StageStatus({ published }: { published: boolean }) {
  return (
    <span className={`stage-status ${published ? "is-published" : "is-draft"}`}>
      <span className="stage-dot" aria-hidden="true" />
      {published ? "Yayında" : "Taslak"}
    </span>
  );
}
