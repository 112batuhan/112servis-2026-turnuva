import type { ReactNode } from "react";
import "./Switch.css";

interface Props {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Says what it switches; also its accessible name. */
  children: ReactNode;
  title?: string;
  className?: string;
}

/** An on/off setting: a small track whose thumb slides over and turns frost when on, its label beside it. */
export function Switch({ checked, onChange, disabled, children, title, className }: Props) {
  return (
    <label
      className={["switch", checked && "is-on", className].filter(Boolean).join(" ")}
      title={title}
    >
      <input
        type="checkbox"
        role="switch"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.target.checked)}
      />
      <span>{children}</span>
    </label>
  );
}
