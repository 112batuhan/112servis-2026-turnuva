import { useState } from "react";
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
