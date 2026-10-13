import { useId, type CSSProperties, type HTMLAttributes, type ReactNode } from "react";
import { Icon } from "./Icon";
import "./PoolSection.css";

interface Props extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  title: ReactNode;
  /** Small text after the title, e.g. "4 slots". */
  meta?: ReactNode;
  /** Buttons on the right of the header (outside the toggle). */
  actions?: ReactNode;
  /** Category colour; tints the header. */
  color?: string;
  expanded: boolean;
  onToggle: () => void;
  /** Highlights the section while a map is dragged over it. */
  isDropTarget?: boolean;
  children: ReactNode;
}

/** A collapsible block of the pool: the generic pool or one category. */
export function PoolSection({
  title,
  meta,
  actions,
  color,
  expanded,
  onToggle,
  isDropTarget,
  children,
  className = "",
  ...rest
}: Props) {
  const bodyId = useId();
  const style = color ? ({ "--section-color": color } as CSSProperties) : undefined;

  return (
    <section
      className={`pool-section ${color ? "has-color" : ""} ${isDropTarget ? "is-drop-target" : ""} ${className}`}
      style={style}
      {...rest}
    >
      <div className="pool-section-head">
        <button
          type="button"
          className="pool-section-toggle"
          aria-expanded={expanded}
          aria-controls={bodyId}
          onClick={onToggle}
        >
          <Icon name="chevronDown" size={20} className="pool-section-caret" />
          <span className="pool-section-title">{title}</span>
          {meta && <span className="pool-section-meta">{meta}</span>}
        </button>
        {actions && <div className="pool-section-actions">{actions}</div>}
      </div>
      {expanded && (
        <div id={bodyId} className="pool-section-body">
          {children}
        </div>
      )}
    </section>
  );
}
