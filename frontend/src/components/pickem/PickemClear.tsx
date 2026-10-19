import { Icon } from "../Icon";
import "./PickemClear.css";

/**
 * Takes a saved prediction back: a small × that sits in a line of text without making it taller.
 * Its touch target reaches past it invisibly, so swapping a note for "saved ×" doesn't shift the
 * slider under a finger that's still on it.
 */
export function PickemClear({ label, disabled, onClick }: { label: string; disabled?: boolean; onClick: () => void }) {
  return (
    <button type="button" className="pickem-clear" aria-label={label} title={label} disabled={disabled} onClick={onClick}>
      <Icon name="close" size={14} />
    </button>
  );
}
