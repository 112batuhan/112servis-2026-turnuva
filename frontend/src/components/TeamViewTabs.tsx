import { useState } from "react";
import { readString, writeString } from "../lib/storage";
import { Icon, type IconName } from "./Icon";
import "./bracket/BracketViewTabs.css";

export type TeamView = "cards" | "rows";

const VIEWS: { view: TeamView; label: string; icon: IconName }[] = [
  { view: "cards", label: "Kartlar", icon: "cards" },
  { view: "rows", label: "Satırlar", icon: "rows" },
];

/** The last team view picked under `key`, remembered per browser. */
export function useTeamView(key: string) {
  const storageKey = `team-view:${key}`;
  const [view, setView] = useState<TeamView>(() => {
    const saved = readString(storageKey);
    if (VIEWS.some((v) => v.view === saved)) return saved as TeamView;
    // Phones open on the rows: a card is a whole screen there.
    return window.matchMedia("(max-width: 720px)").matches ? "rows" : "cards";
  });
  const select = (next: TeamView) => {
    setView(next);
    writeString(storageKey, next);
  };
  return [view, select] as const;
}

/** Switches a team list between cards and rows; styled like the bracket's view tabs. */
export function TeamViewTabs({ view, onChange }: { view: TeamView; onChange: (view: TeamView) => void }) {
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
