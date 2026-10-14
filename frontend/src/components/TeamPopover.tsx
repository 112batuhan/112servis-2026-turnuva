import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
  type MouseEvent,
  type ReactNode,
  type SyntheticEvent,
} from "react";
import { createPortal } from "react-dom";
import { useLocation } from "react-router-dom";
import type { Team } from "../api/types";
import { TeamCard } from "./TeamCard";
import "./TeamPopover.css";

/** Space between the name and the card, and kept free at the viewport's edges. */
const GAP = 8;
const MARGIN = 16;

interface Props {
  team: Team;
  /** Added to the name's own class, e.g. to place it in the parent's layout. */
  className?: string;
  children: ReactNode;
}

// Events inside the card still bubble to the name's React ancestors through the portal, where the
// bracket board would start a drag, select the match or capture the pointer. They stop here.
const stop = (e: SyntheticEvent) => e.stopPropagation();

/**
 * A team's name that opens its `TeamCard` when clicked (or Enter/Space), e.g. to reach the
 * players' osu! profiles. The card is rendered into `document.body` with fixed positioning, so it
 * isn't clipped or scaled by the bracket board. It closes on another click on the name, a click
 * outside it, Escape, anything that scrolls the name away from it, or following a link in it.
 */
export function TeamPopover({ team, className, children }: Props) {
  const triggerRef = useRef<HTMLSpanElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);

  const hide = useCallback((refocus = false) => {
    setOpen(false);
    setPosition(null);
    if (refocus) triggerRef.current?.focus();
  }, []);

  // A match in the card's history links to the bracket, which may be the page it's on.
  const { pathname, search } = useLocation();
  useEffect(() => hide(), [pathname, search, hide]);

  const toggle = () => (open ? hide() : setOpen(true));

  // Clicking the name opens the card instead of selecting the match it's in.
  const onClick = (e: MouseEvent) => {
    e.stopPropagation();
    toggle();
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if (e.key !== "Enter" && e.key !== " ") return;
    e.preventDefault();
    e.stopPropagation();
    toggle();
  };

  // Below the name, or above it when there's no room below; kept inside the viewport.
  useLayoutEffect(() => {
    if (!open || !triggerRef.current || !cardRef.current) return;
    const anchor = triggerRef.current.getBoundingClientRect();
    const card = cardRef.current.getBoundingClientRect();
    let top = anchor.bottom + GAP;
    if (top + card.height > window.innerHeight - MARGIN && anchor.top - GAP - card.height >= MARGIN) {
      top = anchor.top - GAP - card.height;
    }
    const left = Math.max(MARGIN, Math.min(anchor.left, window.innerWidth - MARGIN - card.width));
    setPosition({ top, left });
  }, [open]);

  // Once placed, focus moves into the card so its links are next in the tab order.
  useEffect(() => {
    if (position) cardRef.current?.focus({ preventScroll: true });
  }, [position]);

  // Anything that moves the name away from the card closes it, and so does a click outside both.
  useEffect(() => {
    if (!open) return;
    const outside = (e: Event) =>
      !(e.target instanceof Node && (cardRef.current?.contains(e.target) || triggerRef.current?.contains(e.target)));
    const onOutside = (e: Event) => {
      if (outside(e)) hide();
    };
    const onEscape = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape") hide(cardRef.current?.contains(document.activeElement) ?? false);
    };
    const onResize = () => hide();
    window.addEventListener("scroll", onOutside, true);
    window.addEventListener("wheel", onOutside, { passive: true });
    window.addEventListener("pointerdown", onOutside);
    window.addEventListener("resize", onResize);
    window.addEventListener("keydown", onEscape);
    return () => {
      window.removeEventListener("scroll", onOutside, true);
      window.removeEventListener("wheel", onOutside);
      window.removeEventListener("pointerdown", onOutside);
      window.removeEventListener("resize", onResize);
      window.removeEventListener("keydown", onEscape);
    };
  }, [open, hide]);

  // Tabbing out of the card closes it. A null target is the window losing focus, e.g. to the
  // profile tab a link just opened, so the card stays for the other player.
  const onBlur = (e: FocusEvent) => {
    const next = e.relatedTarget;
    if (next instanceof Node && !cardRef.current?.contains(next) && !triggerRef.current?.contains(next)) hide();
  };

  return (
    <>
      <span
        ref={triggerRef}
        className={className ? `team-popover-trigger ${className}` : "team-popover-trigger"}
        role="button"
        tabIndex={0}
        aria-haspopup="dialog"
        aria-expanded={open}
        onClick={onClick}
        onKeyDown={onKeyDown}
      >
        {children}
      </span>
      {open &&
        createPortal(
          <div
            ref={cardRef}
            className="team-popover"
            role="dialog"
            aria-label={team.name}
            tabIndex={-1}
            style={position ? { top: position.top, left: position.left } : { visibility: "hidden" }}
            onBlur={onBlur}
            onPointerDown={stop}
            onPointerMove={stop}
            onPointerUp={stop}
            onMouseDown={stop}
            onClick={stop}
            onDoubleClick={stop}
            onKeyDown={stop}
          >
            <TeamCard team={team} />
          </div>,
          document.body,
        )}
    </>
  );
}
