import {
  useCallback,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
  type RefObject,
} from "react";
import { createPortal } from "react-dom";
import type { AdminUser } from "../../api/types";
import { Icon } from "../../components/Icon";
import { formatDate } from "../../lib/format";
import "./UserBan.css";

/** As the server trims it (`BAN_REASON_MAX` in handlers/admin.rs). */
const BAN_REASON_MAX = 300;

// Kept free at the viewport's edges, and between the icon and its popover.
const MARGIN = 12;
const GAP = 6;

interface Props {
  user: AdminUser;
  /** The host's own row: they can't ban themselves, so it shows nothing. */
  self: boolean;
  busy: boolean;
  /** Each resolves to whether it went through. */
  onBan: (reason: string) => Promise<boolean>;
  onUnban: () => Promise<boolean>;
}

/**
 * A user's ban in the users table: one icon, the same width either way so the column never grows.
 * Muted while they aren't banned, red on a red wash once they are. It opens a popover: the reason
 * to ban them with, or why, by whom and when they were banned with the button that lifts it. The
 * popover floats over the page, so the table's scroll can't clip it.
 */
export function UserBan({ user, self, busy, onBan, onUnban }: Props) {
  const banned = user.banned_at !== null;
  // What the popover shows, fixed when it opens: the list reloads before a ban or an unban closes
  // it, and it shouldn't flip to the other side for that moment.
  const [shows, setShows] = useState<
    { kind: "ban" } | { kind: "details"; reason: string; meta: string } | null
  >(null);
  const open = shows !== null;
  const [reason, setReason] = useState("");
  const triggerRef = useRef<HTMLButtonElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const style = useAnchored(open, triggerRef, popRef);
  const popId = useId();

  const close = useCallback((refocus: boolean) => {
    setShows(null);
    setReason("");
    if (refocus) triggerRef.current?.focus();
  }, []);

  // Once it has its place, the reason field takes the focus (without scrolling the page to it); the
  // details of a ban have nothing to type, so their button does.
  const placed = open && "top" in style;
  useEffect(() => {
    if (!placed) return;
    const target = inputRef.current ?? popRef.current?.querySelector<HTMLElement>("button");
    target?.focus({ preventScroll: true });
  }, [placed]);

  // A click outside it, or Escape, puts it away.
  useEffect(() => {
    if (!open) return;
    const inside = (target: EventTarget | null) =>
      target instanceof Node &&
      (popRef.current?.contains(target) || triggerRef.current?.contains(target));
    const onPointer = (e: PointerEvent) => {
      if (!inside(e.target)) close(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") close(true);
    };
    window.addEventListener("pointerdown", onPointer);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("pointerdown", onPointer);
      window.removeEventListener("keydown", onKey);
    };
  }, [open, close]);

  if (self && !banned) return null;

  const label = banned
    ? `${user.username} banlı: ayrıntılar`
    : `${user.username} kullanıcısını banla`;

  function toggle() {
    if (open) return close(false);
    if (user.banned_at === null) return setShows({ kind: "ban" });
    setShows({
      kind: "details",
      reason: user.ban_reason ?? "",
      meta: [user.banned_by_username, formatDate(user.banned_at)].filter(Boolean).join(" · "),
    });
  }

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`icon-btn user-ban-trigger${banned ? " is-banned" : ""}${open ? " is-open" : ""}`}
        aria-label={label}
        title={banned ? "Banlı" : "Banla"}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-controls={open ? popId : undefined}
        disabled={busy && !open}
        onClick={toggle}
      >
        <Icon name="ban" size={16} />
      </button>
      {shows &&
        createPortal(
          <div
            ref={popRef}
            id={popId}
            role="dialog"
            aria-label={label}
            className={shows.kind === "details" ? "user-ban-pop is-details" : "user-ban-pop"}
            style={style}
            // Tabbing out of it puts it away too.
            onBlur={(e) => {
              const next = e.relatedTarget;
              if (
                next instanceof Node &&
                !popRef.current?.contains(next) &&
                !triggerRef.current?.contains(next)
              ) {
                close(false);
              }
            }}
          >
            {shows.kind === "details" ? (
              <>
                <div className="user-ban-pop-details">
                  <span
                    className={
                      shows.reason ? "user-ban-pop-reason" : "user-ban-pop-reason is-empty"
                    }
                  >
                    {shows.reason || "Sebep belirtilmedi"}
                  </span>
                  <span className="user-ban-pop-meta">{shows.meta}</span>
                </div>
                <button
                  type="button"
                  className="btn btn-secondary btn-sm"
                  disabled={busy}
                  onClick={async () => {
                    if (await onUnban()) close(true);
                  }}
                >
                  <Icon name="undo" size={14} />
                  Banı kaldır
                </button>
              </>
            ) : (
              <form
                className="user-ban-pop-form"
                onSubmit={async (e) => {
                  e.preventDefault();
                  if (await onBan(reason)) close(true);
                }}
              >
                <input
                  ref={inputRef}
                  className="input input-sm user-ban-pop-input"
                  placeholder="Sebep (oyuncuya gösterilir)"
                  aria-label="Ban sebebi"
                  maxLength={BAN_REASON_MAX}
                  value={reason}
                  onChange={(e) => setReason(e.target.value)}
                />
                <button type="submit" className="btn btn-danger btn-sm" disabled={busy}>
                  <Icon name="ban" size={14} />
                  Banla
                </button>
              </form>
            )}
          </div>,
          document.body,
        )}
    </>
  );
}

/**
 * Places the popover under its icon, or over it when there's no room below; aligned to the icon's
 * right edge and kept inside the viewport. Follows the icon while the page scrolls or resizes (a
 * phone's keyboard opening resizes it).
 */
function useAnchored(
  open: boolean,
  triggerRef: RefObject<HTMLElement | null>,
  floatRef: RefObject<HTMLElement | null>,
) {
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);

  const place = useCallback(() => {
    if (!triggerRef.current || !floatRef.current) return;
    const anchor = triggerRef.current.getBoundingClientRect();
    const box = floatRef.current.getBoundingClientRect();
    const below = anchor.bottom + GAP;
    const above = anchor.top - GAP - box.height;
    const top = below + box.height <= window.innerHeight - MARGIN || above < MARGIN ? below : above;
    const left = Math.max(
      MARGIN,
      Math.min(anchor.right - box.width, window.innerWidth - MARGIN - box.width),
    );
    setPosition({ top: Math.max(MARGIN, top), left });
  }, [triggerRef, floatRef]);

  useLayoutEffect(() => {
    if (open) place();
    else setPosition(null);
  }, [open, place]);

  useEffect(() => {
    if (!open) return;
    window.addEventListener("scroll", place, true);
    window.addEventListener("resize", place);
    return () => {
      window.removeEventListener("scroll", place, true);
      window.removeEventListener("resize", place);
    };
  }, [open, place]);

  // Measured hidden first, then shown where it belongs.
  return position ? { top: position.top, left: position.left } : { visibility: "hidden" as const };
}
