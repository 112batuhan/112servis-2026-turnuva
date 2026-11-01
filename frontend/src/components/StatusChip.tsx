import type { ReactNode } from "react";
import { Icon } from "./Icon";
import "./StatusChip.css";

/**
 * Where a match or lobby stands. `waiting`: its time is still to come (`none`: it has no time yet,
 * `potential`: its teams aren't decided); `due`: its time has come without a room; `live`: being
 * played; `done`: concluded; `full`: a lobby with no places left.
 */
export type ChipState = "waiting" | "none" | "potential" | "due" | "live" | "done" | "full";

interface Props {
  state: ChipState;
  children: ReactNode;
  title?: string;
}

/** A match's state as a squared chip in condensed display type, led by a mark that says it at a glance. */
export function StatusChip({ state, children, title }: Props) {
  return (
    <span className={`status-chip is-${state}`} title={title}>
      {state === "done" ? (
        <Icon name="check" size={12} className="status-chip-mark" />
      ) : (
        <span className="status-chip-mark" aria-hidden="true" />
      )}
      {children}
    </span>
  );
}
