import { useEffect, useRef, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { readString, writeString } from "../../lib/storage";
import { Icon, type IconName } from "../Icon";
import "./BracketViewTabs.css";

export type BracketView = "grid" | "list" | "calendar";

const VIEWS: { view: BracketView; label: string; icon: IconName }[] = [
  { view: "grid", label: "Bracket", icon: "bracket" },
  { view: "list", label: "Liste", icon: "list" },
  { view: "calendar", label: "Takvim", icon: "calendar" },
];

/** The last view picked under `key`, remembered per browser. */
export function useBracketView(key: string) {
  const storageKey = `bracket-view:${key}`;
  const [view, setView] = useState<BracketView>(() => {
    const saved = readString(storageKey);
    return VIEWS.some((v) => v.view === saved) ? (saved as BracketView) : "grid";
  });
  const select = (next: BracketView) => {
    setView(next);
    writeString(storageKey, next);
  };
  return [view, select] as const;
}

/**
 * Calls `onLinked` with the match a link opened the page on (`?match=<id>`, see
 * `bracketMatchPath`), then drops the parameter so the same link works again later.
 */
export function useLinkedMatch(onLinked: (id: string) => void) {
  const [params, setParams] = useSearchParams();
  const id = params.get("match");
  const onLinkedRef = useRef(onLinked);
  onLinkedRef.current = onLinked;

  useEffect(() => {
    if (!id) return;
    onLinkedRef.current(id);
    setParams(
      (p) => {
        p.delete("match");
        return p;
      },
      { replace: true },
    );
  }, [id, setParams]);
}

/** Whether match times show as a countdown, remembered per browser under `key`. */
export function useCountdownMode(key: string) {
  const storageKey = `bracket-countdown:${key}`;
  const [on, setOn] = useState(() => readString(storageKey) === "1");
  const toggle = () => {
    setOn(!on);
    writeString(storageKey, on ? "0" : "1");
  };
  return [on, toggle] as const;
}

/** Switches match times between the date and a countdown; styled like the view tabs. */
export function CountdownToggle({ on, onToggle }: { on: boolean; onToggle: () => void }) {
  return (
    <div className="bracket-view-tabs">
      <button type="button" className="bracket-view-tab" aria-pressed={on} onClick={onToggle}>
        <Icon name="timer" />
        Geri sayım
      </button>
    </div>
  );
}

export function BracketViewTabs({ view, onChange }: { view: BracketView; onChange: (view: BracketView) => void }) {
  return (
    <div className="bracket-view-tabs" role="group" aria-label="Görünüm">
      {VIEWS.map((v) => (
        <button
          key={v.view}
          type="button"
          className="bracket-view-tab"
          aria-pressed={view === v.view}
          onClick={() => onChange(v.view)}
        >
          <Icon name={v.icon} />
          {v.label}
        </button>
      ))}
    </div>
  );
}
