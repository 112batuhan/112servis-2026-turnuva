import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type PointerEvent,
} from "react";

/**
 * A press held for `ms` before it counts: the room page's bans and picks, where a slip of the
 * finger shouldn't cost a team its map. A pointer held on the element (a touch that starts
 * scrolling instead lets go), or Space or Enter held while it's focused. `left` is the time still
 * to hold, while it's held; letting go early cancels.
 */
export function useHold(ms: number, onDone: () => void, enabled: boolean) {
  const [holding, setHolding] = useState(false);
  const [left, setLeft] = useState(ms);
  const timer = useRef<number | null>(null);
  const ticker = useRef<number | null>(null);
  const doneRef = useRef(onDone);
  useEffect(() => {
    doneRef.current = onDone;
  });

  const stop = useCallback(() => {
    if (timer.current !== null) window.clearTimeout(timer.current);
    if (ticker.current !== null) window.clearInterval(ticker.current);
    timer.current = null;
    ticker.current = null;
    setHolding(false);
    setLeft(ms);
  }, [ms]);

  const start = useCallback(() => {
    if (!enabled || timer.current !== null) return;
    const began = Date.now();
    setHolding(true);
    setLeft(ms);
    ticker.current = window.setInterval(() => setLeft(Math.max(0, ms - (Date.now() - began))), 100);
    timer.current = window.setTimeout(() => {
      stop();
      doneRef.current();
    }, ms);
  }, [enabled, ms, stop]);

  // Turned off while held (the turn passed): the hold goes.
  useEffect(() => {
    if (!enabled) stop();
  }, [enabled, stop]);
  useEffect(() => stop, [stop]);

  const bind = {
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      if (e.button !== 0) return;
      e.currentTarget.setPointerCapture?.(e.pointerId);
      start();
    },
    onPointerUp: stop,
    onPointerCancel: stop,
    onLostPointerCapture: stop,
    onKeyDown: (e: KeyboardEvent<HTMLElement>) => {
      if ((e.key === " " || e.key === "Enter") && !e.repeat) {
        e.preventDefault();
        start();
      }
    },
    onKeyUp: (e: KeyboardEvent<HTMLElement>) => {
      if (e.key === " " || e.key === "Enter") stop();
    },
    onBlur: stop,
    // A long press on a phone would open the browser's menu over it.
    onContextMenu: (e: { preventDefault: () => void }) => {
      if (enabled) e.preventDefault();
    },
  };

  return { holding, left, bind };
}
