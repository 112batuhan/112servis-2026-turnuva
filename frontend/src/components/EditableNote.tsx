import { useEffect, useRef, useState } from "react";
import "./EditableNote.css";

interface Props {
  value: string;
  /** Accessible name, e.g. "Public note". */
  label: string;
  placeholder: string;
  /** Private notes get a dashed border so they're never mistaken for public text. */
  isPrivate?: boolean;
  onSave: (next: string) => void;
  /** Lets a draggable parent disable dragging while the textarea is open. */
  onEditingChange?: (editing: boolean) => void;
}

// Idle: one truncated line (full text in a tooltip on hover/focus). Click to edit in a
// textarea that grows with its content. Blur or Escape saves.
export function EditableNote({ value, label, placeholder, isPrivate, onSave, onEditingChange }: Props) {
  const [editing, setEditing] = useState(false);
  const [clipped, setClipped] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const textRef = useRef<HTMLSpanElement>(null);

  function setEditingState(on: boolean) {
    setEditing(on);
    onEditingChange?.(on);
  }

  useEffect(() => {
    const el = textareaRef.current;
    if (!editing || !el) return;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
    autoGrow(el);
  }, [editing]);

  // Only show the tooltip when the idle line is actually truncated.
  useEffect(() => {
    const el = textRef.current;
    if (!el) return;
    const measure = () => setClipped(el.scrollWidth > el.clientWidth);
    measure();
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [value, editing]);

  const classes = `note ${isPrivate ? "note-private" : ""}`;

  if (editing) {
    return (
      <textarea
        ref={textareaRef}
        className={`${classes} note-textarea`}
        rows={1}
        defaultValue={value}
        placeholder={placeholder}
        aria-label={label}
        onInput={(e) => autoGrow(e.currentTarget)}
        onKeyDown={(e) => e.key === "Escape" && e.currentTarget.blur()}
        onBlur={(e) => {
          setEditingState(false);
          if (e.target.value !== value) onSave(e.target.value);
        }}
      />
    );
  }

  return (
    <button
      type="button"
      className={`${classes} note-view ${value ? "" : "is-empty"}`}
      aria-label={`${label}: ${value || "empty"}. Edit`}
      onClick={() => setEditingState(true)}
    >
      <span ref={textRef} className="note-view-text">
        {value || placeholder}
      </span>
      {value && clipped && (
        <span className="note-tooltip" aria-hidden="true">
          {value}
        </span>
      )}
    </button>
  );
}

function autoGrow(el: HTMLTextAreaElement) {
  el.style.height = "auto";
  el.style.height = `${el.scrollHeight}px`;
}
