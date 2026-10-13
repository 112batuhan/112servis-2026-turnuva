import type { ButtonHTMLAttributes } from "react";
import { Icon, type IconName } from "./Icon";

interface Props extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> {
  icon: IconName;
  /** Required: icon-only buttons need a name for screen readers. Also shown as a tooltip. */
  label: string;
  tone?: "default" | "danger";
  variant?: "plain" | "outlined" | "round";
  active?: boolean;
  iconSize?: number;
}

export function IconButton({
  icon,
  label,
  tone = "default",
  variant = "plain",
  active = false,
  iconSize = 16,
  className = "",
  ...rest
}: Props) {
  const classes = [
    "icon-btn",
    variant === "outlined" && "icon-btn-outlined",
    variant === "round" && "icon-btn-outlined icon-btn-round",
    tone === "danger" && "icon-btn-danger",
    active && "icon-btn-active",
    className,
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <button type="button" className={classes} aria-label={label} title={label} {...rest}>
      <Icon name={icon} size={iconSize} />
    </button>
  );
}
