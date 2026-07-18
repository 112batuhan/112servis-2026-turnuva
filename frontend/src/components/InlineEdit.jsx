import { useState } from "react";
import "./InlineEdit.css";

// Text with a pencil button to edit it inline. Enter or blur saves (when changed and
// non-empty); Escape cancels. `onSave(next)` receives the trimmed new value.
export default function InlineEdit({ value, onSave, className = "" }) {
  const [editing, setEditing] = useState(false);

  const commit = (raw) => {
    setEditing(false);
    const next = raw.trim();
    if (next && next !== value) onSave(next);
  };

  if (editing) {
    return (
      <input
        className={`inline-edit ${className}`}
        defaultValue={value}
        autoFocus
        onFocus={(e) => e.target.select()}
        onBlur={(e) => commit(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.target.blur();
          else if (e.key === "Escape") {
            e.target.value = value; // restore so blur is a no-op
            e.target.blur();
          }
        }}
        // Keep clicks from bubbling to parents (e.g. a collapsible header / stage tab).
        onClick={(e) => e.stopPropagation()}
      />
    );
  }

  return (
    <span className={`inline-edit-view ${className}`}>
      {value}
      <button
        type="button"
        className="inline-edit-btn"
        onClick={(e) => {
          e.stopPropagation();
          setEditing(true);
        }}
        aria-label="Rename"
        title="Rename"
      >
        <PencilGlyph />
      </button>
    </span>
  );
}

function PencilGlyph() {
  return (
    <svg
      width="13"
      height="13"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M12 20h9" />
      <path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4z" />
    </svg>
  );
}
