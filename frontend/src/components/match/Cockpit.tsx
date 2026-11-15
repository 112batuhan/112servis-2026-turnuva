import {
  useCallback,
  useLayoutEffect,
  useRef,
  useState,
  type KeyboardEvent,
  type ReactNode,
  type RefObject,
} from "react";
import "./Cockpit.css";

// The cockpit shell the referee page and the public match page share: its three panels, and on a
// phone the tab bar that shows one of them at a time.

export type CockpitTab = "pool" | "room" | "chat";

export const TABS: { id: CockpitTab; label: string }[] = [
  { id: "pool", label: "Havuz" },
  { id: "room", label: "Maç" },
  { id: "chat", label: "Sohbet" },
];

/**
 * The cockpit's layout: on a phone one panel at a time under a tab bar; wider, every panel shows
 * and the bar is hidden. The CSS decides which, so this reads it back from the bar. Shared by the
 * referee page and the public match page.
 */
export function useCockpit(loaded: boolean) {
  const [tab, setTab] = useState<CockpitTab>("room");
  const [compact, setCompact] = useState(false);
  const pageRef = useRef<HTMLDivElement>(null);
  const barRef = useRef<HTMLDivElement>(null);
  const tabsRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const page = pageRef.current;
    const bar = barRef.current;
    const tabs = tabsRef.current;
    if (!page || !bar || !tabs) return;
    const update = () => {
      setCompact(tabs.offsetHeight > 0);
      page.style.setProperty("--referee-bar-height", `${bar.offsetHeight}px`);
    };
    update();
    const observer = new ResizeObserver(update);
    observer.observe(bar);
    return () => observer.disconnect();
  }, [loaded]);

  // Picking a tab brings the bar up under the top bar, so the panel gets the whole screen.
  const select = useCallback((next: CockpitTab) => {
    setTab(next);
    const bar = barRef.current;
    if (!bar) return;
    const stuckAt = parseFloat(getComputedStyle(bar).top) || 0;
    const offset = bar.getBoundingClientRect().top - stuckAt;
    if (offset > 1) {
      const smooth = !window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      window.scrollBy({ top: offset, behavior: smooth ? "smooth" : "auto" });
    }
  }, []);

  return { tab, select, compact, pageRef, barRef, tabsRef };
}

export const panelProps = (id: CockpitTab) => ({
  role: "tabpanel",
  id: `referee-panel-${id}`,
  "aria-labelledby": `referee-tab-${id}`,
});

interface CockpitTabsProps {
  tabsRef: RefObject<HTMLDivElement>;
  /** The tab list's accessible name. */
  label: string;
  active: CockpitTab;
  onSelect: (tab: CockpitTab) => void;
  /** Where each panel stands, in one short line. */
  status: Record<CockpitTab, ReactNode>;
}

/**
 * The phone's cockpit tabs. Each one carries where its panel stands (the draft, the score, the
 * chat), so nobody has to switch to look.
 */
export function CockpitTabs({
  tabsRef,
  label: listLabel,
  active,
  onSelect,
  status,
}: CockpitTabsProps) {
  function onKeyDown(e: KeyboardEvent) {
    const at = TABS.findIndex((t) => t.id === active);
    const to =
      e.key === "ArrowRight"
        ? (at + 1) % TABS.length
        : e.key === "ArrowLeft"
          ? (at + TABS.length - 1) % TABS.length
          : e.key === "Home"
            ? 0
            : e.key === "End"
              ? TABS.length - 1
              : null;
    if (to === null) return;
    e.preventDefault();
    onSelect(TABS[to].id);
    document.getElementById(`referee-tab-${TABS[to].id}`)?.focus();
  }

  return (
    <div
      className="referee-tabs"
      role="tablist"
      aria-label={listLabel}
      ref={tabsRef}
      onKeyDown={onKeyDown}
    >
      {TABS.map(({ id, label }) => (
        <button
          key={id}
          type="button"
          role="tab"
          id={`referee-tab-${id}`}
          aria-selected={id === active}
          aria-controls={`referee-panel-${id}`}
          tabIndex={id === active ? 0 : -1}
          className="referee-tab"
          onClick={() => onSelect(id)}
        >
          <span className="referee-tab-label">{label}</span>
          <span className="referee-tab-status">{status[id]}</span>
        </button>
      ))}
    </div>
  );
}
