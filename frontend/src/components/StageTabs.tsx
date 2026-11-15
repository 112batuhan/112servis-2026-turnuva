import type { ReactNode } from "react";
import type { Stage } from "../api/types";
import { bestOfLabel, firstTo } from "../lib/bestOf";
import { useCentredItem } from "../lib/useCentredItem";
import "./StageTabs.css";

/**
 * What the tabs show of a stage: the editor's `Stage`, the public `PublicStage` and pick'em's
 * `PickemStage` all have it.
 */
type TabStage = Pick<Stage, "id" | "name"> & {
  published?: boolean;
  is_qualifier?: boolean;
  best_of?: number | null;
};

/** What a page says of a stage on its rule and under its name, in place of `currentId` and `live`. */
export interface StageMark {
  /**
   * The rule: "lit" in frost with the crystal's glow (current, open, being played), "done" in
   * muted ink (played out), "set" in ink (locked); none on a hairline.
   */
  rule?: "lit" | "done" | "set";
  /** The line under the name. */
  note?: string | null;
  /** Say the note in frost (something is happening there now); by default when the rule is lit. */
  noteLit?: boolean;
}

interface Props<S extends TabStage> {
  stages: S[];
  /** The stage shown; without `onSelect` (a display, like the landing's) none is. */
  selectedId?: string | null;
  /** Pick a stage. Without it the track only shows where the tournament is: no buttons, the current stage centred. */
  onSelect?: (id: string) => void;
  /** A stage about to be picked (hovered, focused): start reading it, so the click finds it there. */
  onPrefetch?: (id: string) => void;
  /** The stage the tournament is in (`state` "current" on /api/public/stages): lit in frost with the crystal's glow, "Şu an" under it. */
  currentId?: string | null;
  /** Matches being played now, by stage (`live` on /api/public/stages): said in frost under the stage's name. */
  live?: ReadonlyMap<string, number>;
  /** The page's own reading of each stage (/stats' scores, pick'em's windows), in place of `currentId` and `live`. */
  mark?: (stage: S) => StageMark;
  /** Say on each stage whether it's published or a draft (editor only). */
  showStatus?: boolean;
  /** The track's accessible name. */
  label?: string;
  /** Extra controls after the track (the pack download, editor buttons). */
  children?: ReactNode;
}

/**
 * The stages in play order as the site's stage track (the landing's, the pools', /stats',
 * pick'em's, the bracket list's, the editor's): a 3px rule over each stage's name and best of, the
 * shown one on the hover wash in ink, the lit one glowing. Wide enough, it lays out as a grid that
 * wraps; on a phone it runs in one line that scrolls sideways, with the shown stage (on the
 * landing, the current one) kept in the middle. In the editor a draft's rule is dashed.
 */
export function StageTabs<S extends TabStage>({
  stages,
  selectedId = null,
  onSelect,
  onPrefetch,
  currentId,
  live,
  mark,
  showStatus,
  label = "Stage'ler",
  children,
}: Props<S>) {
  const centredId = onSelect ? selectedId : (currentId ?? null);
  const navRef = useCentredItem<HTMLElement>("[data-centred]", centredId, stages.length);
  return (
    <div className="stage-bar">
      <nav ref={navRef} className="stage-tabs" aria-label={label}>
        <ol>
          {stages.map((stage) => {
            const m = mark ? mark(stage) : defaultMark(stage, currentId, live);
            const classes = [
              "stage-tab",
              m.rule && `is-${m.rule}`,
              showStatus && stage.published === false && "is-draft",
            ];
            const className = classes.filter(Boolean).join(" ");
            const body = (
              <>
                <span className="stage-tab-head">
                  <span className="stage-tab-name">{stage.name}</span>
                  {stage.best_of != null && !stage.is_qualifier && (
                    <span
                      className="stage-tab-bo"
                      title={`${firstTo(stage.best_of)} map alan kazanır`}
                    >
                      {bestOfLabel(stage.best_of)}
                    </span>
                  )}
                </span>
                <StageNote mark={m} published={showStatus ? stage.published !== false : null} />
              </>
            );
            return (
              <li key={stage.id} data-centred={stage.id === centredId || undefined}>
                {onSelect ? (
                  <button
                    type="button"
                    className={className}
                    aria-pressed={stage.id === selectedId}
                    onClick={() => onSelect(stage.id)}
                    onPointerEnter={onPrefetch && (() => onPrefetch(stage.id))}
                    onFocus={onPrefetch && (() => onPrefetch(stage.id))}
                  >
                    {body}
                  </button>
                ) : (
                  <span
                    className={className}
                    aria-current={stage.id === currentId ? "step" : undefined}
                  >
                    {body}
                  </span>
                )}
              </li>
            );
          })}
        </ol>
      </nav>
      {children && <div className="stage-bar-actions">{children}</div>}
    </div>
  );
}

/** The public pages' reading: the current stage lit with "Şu an", any stage's matches being played said in frost. */
function defaultMark(
  stage: TabStage,
  currentId: string | null | undefined,
  live: ReadonlyMap<string, number> | undefined,
): StageMark {
  const current = stage.id === currentId;
  const playing = live?.get(stage.id) ?? 0;
  const note =
    playing > 0
      ? `${playing} ${stage.is_qualifier ? "lobi" : "maç"} oynanıyor`
      : current
        ? "Şu an"
        : null;
  return { rule: current ? "lit" : undefined, note, noteLit: note !== null };
}

/** What's under the name: the page's note, and in the editor whether the stage is published. */
function StageNote({ mark, published }: { mark: StageMark; published: boolean | null }) {
  if (!mark.note && published === null) return null;
  const lit = mark.noteLit ?? mark.rule === "lit";
  return (
    <span className="stage-tab-note">
      {mark.note && <span className={lit ? "stage-tab-now" : undefined}>{mark.note}</span>}
      {published !== null && <StageStatus published={published} />}
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
