import { useEffect, useState } from "react";
import type { AdminTeamPlayer } from "../api/types";
import "./DiscordMention.css";

interface Props {
  player: Pick<AdminTeamPlayer, "discord_id" | "discord_username">;
  /** Shown when the player has no linked Discord. */
  fallback: string;
}

const COPIED_MS = 1500;

/** A player's Discord name that copies a ready-to-paste mention (<@id>) when clicked. */
export function DiscordMention({ player, fallback }: Props) {
  const { discord_id: discordId, discord_username: username } = player;
  const [state, setState] = useState<"idle" | "copied" | "failed">("idle");

  useEffect(() => {
    if (state === "idle") return;
    const timer = setTimeout(() => setState("idle"), COPIED_MS);
    return () => clearTimeout(timer);
  }, [state]);

  if (!username) return <>{fallback}</>;
  // Without the id there's nothing to mention, so just show the name.
  if (!discordId) return <>@{username}</>;

  async function copy() {
    try {
      await navigator.clipboard.writeText(`<@${discordId}>`);
      setState("copied");
    } catch {
      setState("failed");
    }
  }

  return (
    <button
      type="button"
      className="discord-mention"
      title="Discord etiketini kopyala"
      aria-label={`@${username}: Discord etiketini kopyala`}
      onClick={copy}
    >
      {state === "copied" ? "Kopyalandı" : state === "failed" ? "Kopyalanamadı" : `@${username}`}
      <span className="sr-only" role="status">
        {state === "copied" ? "Etiket kopyalandı" : state === "failed" ? "Etiket kopyalanamadı" : ""}
      </span>
    </button>
  );
}
