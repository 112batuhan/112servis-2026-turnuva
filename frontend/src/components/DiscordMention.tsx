import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { AdminTeamPlayer } from "../api/types";
import { Icon } from "./Icon";
import "./DiscordMention.css";

interface Props {
  player: Pick<AdminTeamPlayer, "discord_id" | "discord_username">;
  /** Shown when the player has no linked Discord. */
  fallback: string;
  /**
   * Just a Discord mark beside the name the row already shows, with the Discord name over it on
   * hover, focus or a copy, for rows a long Discord name would crowd.
   */
  compact?: boolean;
}

const COPIED_MS = 1500;

// Kept free at the viewport's edges, and between the mark and its tip.
const MARGIN = 12;
const GAP = 6;

/** A player's Discord name that copies a ready-to-paste mention (<@id>) when clicked. */
export function DiscordMention({ player, fallback, compact = false }: Props) {
  const { discord_id: discordId, discord_username: username } = player;
  const [state, setState] = useState<"idle" | "copied" | "failed">("idle");

  useEffect(() => {
    if (state === "idle") return;
    const timer = setTimeout(() => setState("idle"), COPIED_MS);
    return () => clearTimeout(timer);
  }, [state]);

  if (!username) return <>{fallback}</>;

  async function copy() {
    try {
      await navigator.clipboard.writeText(`<@${discordId}>`);
      setState("copied");
    } catch {
      setState("failed");
    }
  }

  if (compact)
    return (
      <CompactMention username={username} copyable={!!discordId} state={state} onCopy={copy} />
    );

  // Without the id there's nothing to mention, so just show the name.
  if (!discordId) return <>@{username}</>;

  return (
    <button
      type="button"
      className="discord-mention"
      title="Discord etiketini kopyala"
      aria-label={`@${username}: Discord etiketini kopyala`}
      onClick={copy}
    >
      {state === "copied" ? "Kopyalandı" : state === "failed" ? "Kopyalanamadı" : `@${username}`}
      <Status state={state} />
    </button>
  );
}

function Status({ state }: { state: "idle" | "copied" | "failed" }) {
  return (
    <span className="sr-only" role="status">
      {state === "copied" ? "Etiket kopyalandı" : state === "failed" ? "Etiket kopyalanamadı" : ""}
    </span>
  );
}

interface CompactProps {
  username: string;
  copyable: boolean;
  state: "idle" | "copied" | "failed";
  onCopy: () => void;
}

/**
 * The Discord mark as a button. Its tip (over it, or under it near the top of the viewport) names
 * the account and says what a click does, then that it's done; on touch, where nothing hovers, the
 * tip shows on the tap that copies.
 */
function CompactMention({ username, copyable, state, onCopy }: CompactProps) {
  const [hovered, setHovered] = useState(false);
  const [position, setPosition] = useState<{ top: number; left: number } | null>(null);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const tipRef = useRef<HTMLSpanElement>(null);
  const open = hovered || state !== "idle";

  useLayoutEffect(() => {
    if (!open || !triggerRef.current || !tipRef.current) {
      setPosition(null);
      return;
    }
    const anchor = triggerRef.current.getBoundingClientRect();
    const tip = tipRef.current.getBoundingClientRect();
    let top = anchor.top - GAP - tip.height;
    if (top < MARGIN) top = anchor.bottom + GAP;
    const centred = anchor.left + anchor.width / 2 - tip.width / 2;
    const left = Math.max(MARGIN, Math.min(centred, window.innerWidth - MARGIN - tip.width));
    setPosition({ top, left });
  }, [open, state]);

  // Without the id there's nothing to mention, so the tip only names the account.
  const hint = !copyable
    ? null
    : state === "copied"
      ? "Etiket kopyalandı"
      : state === "failed"
        ? "Kopyalanamadı"
        : "Etiketi kopyala";

  return (
    <>
      <button
        ref={triggerRef}
        type="button"
        className={`discord-mention-mark${state !== "idle" ? ` is-${state}` : ""}`}
        aria-label={copyable ? `Discord: @${username}, etiketi kopyala` : `Discord: @${username}`}
        aria-disabled={!copyable || undefined}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        onFocus={() => setHovered(true)}
        onBlur={() => setHovered(false)}
        onClick={copyable ? onCopy : undefined}
      >
        <Icon name={state === "copied" ? "check" : "discord"} size={14} />
        <Status state={state} />
      </button>
      {open &&
        createPortal(
          <span
            ref={tipRef}
            className="discord-mention-tip"
            role="tooltip"
            style={position ? { top: position.top, left: position.left } : { visibility: "hidden" }}
          >
            <span className="discord-mention-tip-name">@{username}</span>
            {hint && (
              <span className={`discord-mention-tip-hint${state !== "idle" ? ` is-${state}` : ""}`}>
                {hint}
              </span>
            )}
          </span>,
          document.body,
        )}
    </>
  );
}
