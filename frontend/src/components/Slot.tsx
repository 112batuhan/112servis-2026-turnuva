import type { HTMLAttributes, ReactNode } from "react";
import "./Slot.css";

interface Props extends HTMLAttributes<HTMLDivElement> {
  number: number;
  /** Shown instead of "#number", e.g. the slot's pick code "NM1". */
  label?: string;
  isDropTarget?: boolean;
  children: ReactNode;
}

/** One numbered position inside a category. */
export function Slot({ number, label, isDropTarget, children, className = "", ...rest }: Props) {
  return (
    <div className={`slot ${isDropTarget ? "is-drop-target" : ""} ${className}`} {...rest}>
      <span className="slot-number" aria-label={label ?? `Slot ${number}`}>
        {label ?? `#${number}`}
      </span>
      <div className="slot-body">{children}</div>
    </div>
  );
}

export function EmptySlot({ children }: { children: ReactNode }) {
  return <div className="slot-empty">{children}</div>;
}
