import { useLayoutEffect, useRef } from "react";

/**
 * Where a track scrolls sideways (a phone), keep one of its items in the middle: the item
 * `selector` finds, re-centred whenever `key` names another one or `count` changes. It jumps there
 * when the track first shows or turns, and glides when `key` moves on (a stage picked, the current
 * stage advancing). Used by the stage tracks (`StageTabs`, the landing's).
 */
export function useCentredItem<T extends HTMLElement>(
  selector: string,
  key: string | null,
  count: number,
) {
  const ref = useRef<T>(null);
  // The key centred last: a different one glides; the first, or the same again, jumps.
  const centred = useRef<string | null | undefined>(undefined);
  const shown = count > 0;

  useLayoutEffect(() => {
    const track = ref.current;
    if (!track) return;
    const moved = centred.current !== undefined && centred.current !== key;
    centred.current = key;
    const glide = moved && !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    centre(track, selector, glide ? "smooth" : "auto");
  }, [selector, key, count]);

  // The track only exists once there's something on it, so watch its size from then.
  useLayoutEffect(() => {
    const track = ref.current;
    if (!shown || !track || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(() => centre(track, selector, "auto"));
    observer.observe(track);
    return () => observer.disconnect();
  }, [selector, shown]);

  return ref;
}

function centre(track: HTMLElement, selector: string, behavior: ScrollBehavior) {
  if (track.scrollWidth <= track.clientWidth) return;
  const item = track.querySelector<HTMLElement>(selector);
  if (!item) return;
  const box = track.getBoundingClientRect();
  const at = item.getBoundingClientRect();
  track.scrollTo({
    left: track.scrollLeft + at.left - box.left - (box.width - at.width) / 2,
    behavior,
  });
}
