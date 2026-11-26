import { useEffect, useLayoutEffect, useRef, type UIEvent } from "react";

/** How close to the end still counts as reading the latest line. */
const NEAR_END = 32;

/**
 * Keeps a chat log on its latest line unless the reader has scrolled up to read. Wider than a phone
 * the log scrolls inside its panel; on a phone the panel takes its full height and the page is the
 * one scroller (components/match/Cockpit.css), so there it's the page that follows. Which one it is
 * comes from the log's CSS. A hidden log can't scroll, so coming back into view catches up.
 *
 * `change` is anything that moves the end: the line count, the room shown.
 */
export function useFollowLog(visible: boolean, change: unknown) {
  const ref = useRef<HTMLDivElement>(null);
  const atEnd = useRef(true);

  // On a phone the page scrolls: the reader is at the end while the foot of the chat's panel (the
  // latest line, and the input under it) is on screen.
  useEffect(() => {
    if (!visible) return;
    const onScroll = () => {
      const el = ref.current;
      if (el && !scrollsItself(el)) atEnd.current = belowScreen(el) < NEAR_END;
    };
    window.addEventListener("scroll", onScroll, { passive: true });
    return () => window.removeEventListener("scroll", onScroll);
  }, [visible]);

  // Brings the panel's foot to the bottom of the screen, no further: a chat shorter than the screen
  // keeps its head in view.
  useLayoutEffect(() => {
    const el = ref.current;
    if (!visible || !el || !atEnd.current) return;
    if (scrollsItself(el)) el.scrollTop = el.scrollHeight;
    else if (belowScreen(el) > 0) window.scrollBy({ top: belowScreen(el) });
  }, [visible, change]);

  /** The log's own scroll, wider than a phone. */
  const onScroll = (e: UIEvent<HTMLElement>) => {
    const el = e.currentTarget;
    if (scrollsItself(el))
      atEnd.current = el.scrollHeight - el.scrollTop - el.clientHeight < NEAR_END;
  };

  /** Sending a line means reading the latest. */
  const toEnd = () => {
    atEnd.current = true;
  };

  return { ref, onScroll, toEnd };
}

function scrollsItself(el: HTMLElement) {
  return getComputedStyle(el).overflowY !== "visible";
}

/** How far the foot of the log's panel is below the bottom of the screen. */
function belowScreen(el: HTMLElement) {
  const panel = el.parentElement ?? el;
  return panel.getBoundingClientRect().bottom - window.innerHeight;
}
