import { useLayoutEffect, useRef, useState, type SyntheticEvent } from "react";
import { createPortal } from "react-dom";
import type { RefereeViewer } from "../../api/types";
import { Icon } from "../Icon";
import "./RefereeWatchers.css";

// Kept free at the viewport's edges, and between the badge and the list.
const MARGIN = 12;
const GAP = 6;

// Inside a grid card or a list row: a click here shouldn't select the match, start a
// drag or nudge the card.
const stop = (e: SyntheticEvent) => e.stopPropagation();

/**
 * Who has the match's referee page open right now, from the live socket: an eye with the count,
 * and the names over it on hover, focus or a tap. Renders nothing while nobody does.
 */
export function RefereeWatchers({ viewers }: { viewers: RefereeViewer[] | undefined }) {
  const [hovered, setHovered] = useState(false);
  const [pinned, setPinned] = useState(false);
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const open = (hovered || pinned) && !!viewers?.length;

  // Under the badge, or over it when there's no room below; kept inside the viewport.
  useLayoutEffect(() => {
    if (!open || !triggerRef.current || !listRef.current) {
      setPosition(null);
      return;
    }
    const anchor = triggerRef.current.getBoundingClientRect();
    const list = listRef.current.getBoundingClientRect();
    let top = anchor.bottom + GAP;
    if (top + list.height > window.innerHeight - MARGIN)
      top = Math.max(MARGIN, anchor.top - GAP - list.height);
    const left = Math.max(MARGIN, Math.min(anchor.left, window.innerWidth - MARGIN - list.width));
    setPosition({ top, left });
  }, [open, viewers]);

  if (!viewers || viewers.length === 0) return null;
  const names = viewers.map((v) => v.username).join(", ");

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="referee-watchers"
        aria-label={`Hakem sayfasında: ${names}`}
        aria-expanded={open}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onFocus={() => setHovered(true)}
        onBlur={() => {
          setHovered(false);
          setPinned(false);
        }}
        onClick={(e) => {
          stop(e);
          setPinned((p) => !p);
        }}
        onPointerDown={stop}
        onKeyDown={(e) => {
          if (e.key === "Escape") setPinned(false);
          stop(e);
        }}
      >
        <Icon name="eye" size={12} />
        {viewers.length}
      </button>
      {open &&
        createPortal(
          <div
            ref={listRef}
            className="referee-watchers-list"
            role="tooltip"
            style={position ? { top: position.top, left: position.left } : { visibility: "hidden" }}
          >
            <span className="referee-watchers-title">Hakem sayfasında</span>
            <ul>
              {viewers.map((v) => (
                <li key={v.id}>
                  <img src={v.avatar_url} alt="" />
                  {v.username}
                </li>
              ))}
            </ul>
          </div>,
          document.body,
        )}
    </>
  );
}
