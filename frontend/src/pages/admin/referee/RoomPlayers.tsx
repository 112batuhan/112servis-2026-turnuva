import { useEffect, useState } from "react";
import { errorMessage } from "../../../api/client";
import { pingPlayer } from "../../../api/discord";
import type { AdminTeamPlayer } from "../../../api/types";
import { DiscordMention } from "../../../components/DiscordMention";
import { IconButton } from "../../../components/IconButton";
import { staffRole } from "../../../lib/roomStaff";
import { botBlocker, type RefereeRoomState } from "./useRefereeRoom";
import { PlayersView } from "../../../components/match/PlayersView";

/**
 * The referee's view of the room's players: each with their Discord (which pings them in the
 * match channel), the slot number that opens the slot picker, and the invite button for those
 * not in the room yet.
 */
export function RoomPlayers({ referee }: { referee: RefereeRoomState }) {
  const { room, roster, lobby, findRoster, actions, botReady, canRun } = referee;
  if (!room) return null;
  const concluded = room.concluded_at !== null;
  const ready = botReady && !lobby.closed;
  const why = botBlocker(referee) ?? "Oda kapandı";
  return (
    <PlayersView
      kind={room.kind}
      teams={room.teams}
      roster={roster}
      lobby={lobby}
      findRoster={findRoster}
      finished={concluded}
      showMisplaced
      staffRoleOf={(slot) => staffRole(referee.staff, slot)}
      kick={
        canRun && !concluded && referee.staff !== null
          ? { enabled: ready, disabledReason: why, one: actions.kick, all: actions.kickOutsiders }
          : undefined
      }
      unreadModsHint="Modları henüz okunmadı; Odayı yenile ile okunur"
      slotControl={canRun && !concluded ? { enabled: ready, disabledReason: why, place: actions.place } : undefined}
      renderContact={(player) =>
        room.discord_channel && canRun ? (
          <DiscordPing matchId={room.id} player={player} />
        ) : (
          <DiscordMention player={player} fallback="Discord yok" />
        )
      }
      renderAction={(player, slot) =>
        slot ? null : (
          <IconButton
            icon="userPlus"
            label={`${player.username} oyuncusunu odaya davet et`}
            disabled={!ready}
            title={ready ? undefined : why}
            onClick={() => actions.invite(player.osu_id)}
          />
        )
      }
    />
  );
}

const PING_NOTE_MS = 2500;

/**
 * A player's Discord name that has the bot ping them in the match's Discord channel when
 * clicked, to call them into the room.
 */
function DiscordPing({ matchId, player }: { matchId: string; player: AdminTeamPlayer }) {
  const [state, setState] = useState<{ kind: "idle" | "busy" | "done" } | { kind: "failed"; error: string }>({
    kind: "idle",
  });

  useEffect(() => {
    if (state.kind !== "done" && state.kind !== "failed") return;
    const timer = setTimeout(() => setState({ kind: "idle" }), PING_NOTE_MS);
    return () => clearTimeout(timer);
  }, [state]);

  if (!player.discord_username) return <>Discord yok</>;
  if (player.discord_in_guild === false) {
    return (
      <span className="room-player-discord-left" title="Turnuvanın Discord sunucusundan ayrıldı">
        @{player.discord_username} ⚠<span className="sr-only"> (sunucuda değil)</span>
      </span>
    );
  }

  async function ping() {
    setState({ kind: "busy" });
    try {
      await pingPlayer(matchId, player.id);
      setState({ kind: "done" });
    } catch (e) {
      setState({ kind: "failed", error: errorMessage(e) });
    }
  }

  return (
    <button
      type="button"
      className="room-player-ping"
      title={state.kind === "failed" ? state.error : "Maçın Discord kanalında etiketle"}
      aria-label={`@${player.discord_username}: maçın Discord kanalında etiketle`}
      disabled={state.kind === "busy"}
      onClick={ping}
    >
      {state.kind === "done" ? "Etiketlendi" : state.kind === "failed" ? "Etiketlenemedi" : `@${player.discord_username}`}
      <span className="sr-only" role="status">
        {state.kind === "done" ? "Oyuncu etiketlendi" : state.kind === "failed" ? state.error : ""}
      </span>
    </button>
  );
}
