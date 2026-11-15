import { useState } from "react";
import { IconButton } from "./IconButton";
import "./InlineEdit.css";

interface Props {
  value: string;
  /** Called with the trimmed value, only when it changed and isn't empty. */
  onSave: (next: string) => void;
  /** What is being renamed, for the button label ("Rename stage"). */
  label: string;
  maxLength?: number;
}

/** Text with a pencil button. Enter or blur saves, Escape cancels. */
export function InlineEdit({ value, onSave, label, maxLength }: Props) {
  const [editing, setEditing] = useState(false);

  function commit(raw: string) {
    setEditing(false);
    const next = raw.trim();
    if (next && next !== value) onSave(next);
  }

  if (editing) {
    return (
      <input
        className="inline-edit-input"
        defaultValue={value}
        aria-label={label}
        maxLength={maxLength}
        autoFocus
        onFocus={(e) => e.target.select()}
        onBlur={(e) => commit(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          if (e.key === "Escape") {
            e.currentTarget.value = value;
            e.currentTarget.blur();
          }
        }}
      />
    );
  }

  return (
    <span className="inline-edit">
      <span className="inline-edit-text">{value}</span>
      <IconButton
        icon="pencil"
        label={label}
        iconSize={14}
        className="inline-edit-btn"
        onClick={() => setEditing(true)}
      />
    </span>
  );
}
