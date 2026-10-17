import { useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import { useAuth } from "../../auth/AuthContext";
import { Icon } from "../../components/Icon";
import type { Spectators } from "../../live/LiveContext";
import "./MatchSpectators.css";

// Kept free at the viewport's edges, and between the button and the list.
const MARGIN = 12;
const GAP = 6;

/**
 * Who is watching the live match with you: an eye with the count beside the header's "Canlı", and on a click
 * the signed-in spectators by their osu! name (you first), then how many more aren't signed in.
 */
export function MatchSpectators({ spectators }: { spectators: Spectators }) {
  const { user } = useAuth();
  const [open, setOpen] = useState(false);
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const listId = useId();

  const { viewers, guests } = spectators;
  const count = viewers.length + guests;
  const self = user ? viewers.find((v) => v.id === user.id) : undefined;
  const named = self ? [self, ...viewers.filter((v) => v !== self)] : viewers;

  // Under the button, or over it when there's no room below; kept inside the viewport.
  useLayoutEffect(() => {
    if (!open) {
      setPosition(null);
      return;
    }
    const place = () => {
      if (!triggerRef.current || !listRef.current) return;
      const anchor = triggerRef.current.getBoundingClientRect();
      const list = listRef.current.getBoundingClientRect();
      let top = anchor.bottom + GAP;
      if (top + list.height > window.innerHeight - MARGIN) top = Math.max(MARGIN, anchor.top - GAP - list.height);
      const left = Math.max(MARGIN, Math.min(anchor.left, window.innerWidth - MARGIN - list.width));
      setPosition({ top, left });
    };
    place();
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [open, viewers, guests]);

  // A click anywhere else or Escape closes it; Escape hands focus back to the button.
  useEffect(() => {
    if (!open) return;
    const onPointer = (e: PointerEvent) => {
      const target = e.target as Node;
      if (!triggerRef.current?.contains(target) && !listRef.current?.contains(target)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== "Escape") return;
      setOpen(false);
      triggerRef.current?.focus();
    };
    document.addEventListener("pointerdown", onPointer);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("pointerdown", onPointer);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  if (count === 0) return null;

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className="match-spectators"
        aria-label={`${count} izleyici`}
        aria-expanded={open}
        aria-controls={open ? listId : undefined}
        onClick={() => setOpen((o) => !o)}
      >
        <Icon name="eye" size={15} />
        <span className="match-spectators-count num">{count}</span>
        <span className="match-spectators-label">izleyici</span>
      </button>
      {open && (
        <div
          ref={listRef}
          id={listId}
          className="match-spectators-list"
          role="region"
          aria-label="İzleyenler"
          style={position ? { top: position.top, left: position.left } : { visibility: "hidden" }}
        >
          <p className="match-spectators-head">
            <span className="num">{count}</span> kişi izliyor
          </p>
          {named.length > 0 && (
            <ul>
              {named.map((v) => (
                <li key={v.id}>
                  <img src={v.avatar_url} alt="" loading="lazy" />
                  <span className="match-spectators-name">{v.username}</span>
                  {v === self && <span className="match-spectators-self">sen</span>}
                </li>
              ))}
            </ul>
          )}
          {guests > 0 && (
            <p className="match-spectators-guests">
              {named.length > 0 ? "+" : ""}
              <span className="num">{guests}</span> giriş yapmamış izleyici
            </p>
          )}
          <p className="match-spectators-note">osu! ile giriş yapanlar adıyla görünür.</p>
        </div>
      )}
    </>
  );
}
