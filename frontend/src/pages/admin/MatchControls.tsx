import { useState } from "react";
import type { MatchRoom, Player } from "../../api/types";
import { IconButton } from "../../components/IconButton";
import "./MatchControls.css";

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

interface RoomsEditorProps {
  rooms: MatchRoom[];
  /** Whether the viewer can add and remove rooms; otherwise just lists them. */
  editable: boolean;
  disabled: boolean;
  onAdd: (link: string) => void;
  onRemove: (mpId: number) => void;
}

/**
 * A match's osu! multiplayer rooms, oldest first. Usually one; when a room breaks
 * mid-match the referee adds the new one and the old one stays listed.
 */
export function RoomsEditor({ rooms, editable, disabled, onAdd, onRemove }: RoomsEditorProps) {
  const [draft, setDraft] = useState("");

  return (
    <div className="rooms-editor">
      {rooms.length === 0 && !editable && <span className="muted">—</span>}
      {rooms.length > 0 && (
        <ol className="rooms-editor-list">
          {rooms.map((room, i) => (
            <li key={room.mp_id}>
              {rooms.length > 1 && <span className="muted">{i + 1}.</span>}
              <a href={room.url} target="_blank" rel="noopener noreferrer">
                {room.url.replace(/^https:\/\//, "")}
              </a>
              {editable && (
                <IconButton
                  icon="close"
                  tone="danger"
                  label={`${room.mp_id} numaralı odayı maçtan çıkar`}
                  disabled={disabled}
                  onClick={() => {
                    if (window.confirm("Bu oda maçtan çıkarılsın mı? Kayıtlı sohbeti silinmez.")) onRemove(room.mp_id);
                  }}
                />
              )}
            </li>
          ))}
        </ol>
      )}
      {editable && (
        <form
          className="inline-form"
          onSubmit={(e) => {
            e.preventDefault();
            if (!draft.trim()) return;
            onAdd(draft.trim());
            setDraft("");
          }}
        >
          <input
            className="input input-sm"
            placeholder="osu.ppy.sh/community/matches/…"
            aria-label={rooms.length > 0 ? "Yeni oda linki" : "Maç linki"}
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
          />
          <button type="submit" className="btn btn-secondary btn-sm" disabled={disabled || !draft.trim()}>
            {rooms.length > 0 ? "Oda ekle" : "Kaydet"}
          </button>
        </form>
      )}
    </div>
  );
}
