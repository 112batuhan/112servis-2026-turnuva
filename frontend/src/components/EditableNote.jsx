import { useEffect, useRef, useState } from "react";

// A note field that fits inside a tight map card yet handles arbitrarily long text.
//   - Idle: shows one truncated line; hovering reveals a styled tooltip with the full text.
//   - Editing (click): turns into a textarea that auto-grows to fit its content, so there
//     is no manual resizing and no scrollbar. Blur saves and collapses it back.
//   - Read-only (no `onSave`): just the idle view + tooltip, used on the public page.
// `onEditingChange(bool)` lets the parent card suspend dragging while a note is edited.
export default function EditableNote({ value, placeholder, hidden, onSave, onEditingChange }) {
  const [editing, setEditing] = useState(false);
  const [clipped, setClipped] = useState(false);
  const ref = useRef(null);
  const textRef = useRef(null);

  const autoGrow = (el) => {
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  };

  // On entering edit mode, focus the textarea, drop the caret at the end, and size it.
  useEffect(() => {
    if (!editing || !ref.current) return;
    const el = ref.current;
    el.focus();
    el.setSelectionRange(el.value.length, el.value.length);
    autoGrow(el);
  }, [editing]);

  // Only show the tooltip when the idle line is actually truncated. Re-measure when the
  // text changes and whenever the box is resized (window/layout changes).
  useEffect(() => {
    const el = textRef.current;
    if (!el) return;
    const measure = () => setClipped(el.scrollWidth > el.clientWidth);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(el);
    return () => ro.disconnect();
  }, [value, editing]);

  const setEdit = (on) => {
    setEditing(on);
    onEditingChange?.(on);
  };

  const cls = `note ${hidden ? "note-hidden" : ""}`;

  if (editing) {
    return (
      <textarea
        ref={ref}
        className={`${cls} note-edit`}
        rows={1}
        defaultValue={value}
        placeholder={placeholder}
        onInput={(e) => autoGrow(e.target)}
        onKeyDown={(e) => e.key === "Escape" && e.target.blur()}
        onBlur={(e) => {
          setEdit(false);
          onSave(e.target.value);
        }}
      />
    );
  }

  // Read-only view has nothing to show if empty; editable view shows the placeholder.
  if (!onSave && !value) return null;

  return (
    <div
      className={`${cls} note-view ${value ? "" : "note-view-empty"}`}
      onMouseDown={
        onSave
          ? (e) => {
              e.preventDefault(); // keep the draggable card from grabbing this gesture
              setEdit(true);
            }
          : undefined
      }
    >
      <span ref={textRef} className="note-view-text">
        {value || placeholder}
      </span>
      {value && clipped && <span className="note-tip">{value}</span>}
    </div>
  );
}
