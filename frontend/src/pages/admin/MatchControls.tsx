import { useEffect, useState } from "react";
import type { Player } from "../../api/types";

// Controls shared by the qualifier and bracket admin pages.

interface StaffSelectProps {
  label: string;
  /** Shown for "nobody". */
  emptyLabel: string;
  current: Pick<Player, "id" | "username"> | null;
  options: Player[];
  disabled: boolean;
  onSelect: (userId: string | null) => void;
}

/** Hosts give a match's referee or stream to a staff member. */
export function StaffSelect({ label, emptyLabel, current, options, disabled, onSelect }: StaffSelectProps) {
  // Someone who has since lost their staff role keeps showing until replaced.
  const all = current && !options.some((r) => r.id === current.id) ? [current, ...options] : options;
  return (
    <select
      className="select select-sm"
      value={current?.id ?? ""}
      disabled={disabled}
      aria-label={label}
      onChange={(e) => onSelect(e.target.value || null)}
    >
      <option value="">{emptyLabel}</option>
      {all.map((r) => (
        <option key={r.id} value={r.id}>
          {r.username}
        </option>
      ))}
    </select>
  );
}

interface LinkEditorProps {
  value: string | null;
  disabled: boolean;
  onSave: (next: string) => void;
}

/** The osu! multiplayer room link; saving it empty clears it. */
export function LinkEditor({ value, disabled, onSave }: LinkEditorProps) {
  const [draft, setDraft] = useState(value ?? "");
  useEffect(() => setDraft(value ?? ""), [value]);
  const changed = draft.trim() !== (value ?? "");

  return (
    <form
      className="inline-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (changed) onSave(draft.trim());
      }}
    >
      <input
        className="input input-sm"
        placeholder="osu.ppy.sh/community/matches/…"
        aria-label="Maç linki"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
      />
      <button type="submit" className="btn btn-secondary btn-sm" disabled={disabled || !changed}>
        Kaydet
      </button>
    </form>
  );
}
