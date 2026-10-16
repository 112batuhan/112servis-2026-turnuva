import { useEffect, useState } from "react";
import { errorMessage } from "../../../api/client";
import { pingPlayer } from "../../../api/discord";
import type { AdminTeamPlayer, Team } from "../../../api/types";
import { DiscordMention } from "../../../components/DiscordMention";
import { IconButton } from "../../../components/IconButton";
import { isFreemod, sameName, splitMods, type RoomSlot, type SlotTeam } from "../../../lib/bancho";
import { formatRank } from "../../../lib/format";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./RoomPlayers.css";

const STATUS_LABELS: Record<RoomSlot["status"], string> = {
  ready: "Hazır",
  not_ready: "Hazır değil",
  no_map: "Map yok",
};

/**
 * The match's players where the room has them: each team under a rule in its colour (red then
 * blue side by side in a bracket match, every lobby team in a grid), each player with their slot,
 * whether they're ready, their mods and, while a map is played, their score; their Discord (which
 * pings them in the match channel) and the button to invite them or move them to the other
 * colour. Whoever is in the room without being in the match is listed after.
 */
export function RoomPlayers({ referee }: { referee: RefereeRoomState }) {
  const { room, roster, lobby, findRoster } = referee;
  if (!room) return null;
  const colorOf = (team: Team<AdminTeamPlayer>) => roster.find((r) => r.team.id === team.id)?.color ?? null;
  // A bracket match's red team first.
  const teams =
    room.kind === "bracket"
      ? [...room.teams].sort((a, b) => Number(colorOf(a) === "blue") - Number(colorOf(b) === "blue"))
      : room.teams;
  const strangers = lobby.slots.filter((s) => !findRoster(s));

  return (
    <section className="room-players" aria-label="Oyuncular">
      {teams.length === 0 && <p className="muted">Bu maçta henüz takım yok.</p>}
      <div className={`room-players-teams${room.kind === "bracket" ? " is-versus" : ""}`}>
        {teams.map((team) => (
          <TeamPlayers key={team.id} team={team} color={colorOf(team)} referee={referee} />
        ))}
      </div>
      {strangers.length > 0 && (
        <div className="room-players-strangers">
          <h3 className="room-players-team-name">Maçın oyuncusu değil</h3>
          <ul className="room-players-list">
            {strangers.map((slot) => (
              <StrangerRow key={slot.slot} slot={slot} referee={referee} />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

function TeamPlayers({
  team,
  color,
  referee,
}: {
  team: Team<AdminTeamPlayer>;
  color: SlotTeam | null;
  referee: RefereeRoomState;
}) {
  const { lobby } = referee;
  const slots = team.players.map((p) => lobby.slots.find((s) => s.userId === p.osu_id || sameName(s.username, p.username)));
  const inRoom = slots.filter(Boolean).length;
  return (
    <div className={`room-players-team${color ? ` is-team-${color}` : ""}`}>
      <h3 className="room-players-team-name">
        <span className="room-players-team-title">{team.name}</span>
        <span className="room-players-team-count">
          {inRoom}/{team.players.length} odada
        </span>
      </h3>
      <ul className="room-players-list">
        {team.players.map((p, i) => (
          <PlayerRow key={p.id} player={p} slot={slots[i]} color={color} referee={referee} />
        ))}
      </ul>
    </div>
  );
}

function PlayerRow({
  player,
  slot,
  color,
  referee,
}: {
  player: AdminTeamPlayer;
  slot: RoomSlot | undefined;
  color: SlotTeam | null;
  referee: RefereeRoomState;
}) {
  const { lobby, actions, inChannel, room, canRun } = referee;
  const wrongTeam = slot && color && slot.team && slot.team !== color;
  return (
    <li className={`room-player${slot ? ` is-${slot.status}` : " is-away"}${wrongTeam ? " is-wrong" : ""}`}>
      <span className="room-player-slot" title={slot ? `Odada ${slot.slot}. slot` : "Odada değil"}>
        {slot ? slot.slot : "–"}
      </span>
      <img className="room-player-avatar" src={player.avatar_url} alt="" loading="lazy" />
      <span className="room-player-text">
        <span className="room-player-name">
          <a href={`https://osu.ppy.sh/users/${player.osu_id}`} target="_blank" rel="noopener noreferrer">
            {player.username}
          </a>
          {slot?.host && <span className="badge">Host</span>}
        </span>
        <span className="room-player-meta">
          {wrongTeam && <span className="room-player-wrong">Yanlış takımda · </span>}
          {formatRank(player.global_rank)} ·{" "}
          {room?.discord_channel && canRun ? (
            <DiscordPing matchId={room.id} player={player} />
          ) : (
            <DiscordMention player={player} fallback="Discord yok" />
          )}
        </span>
      </span>
      {slot ? <SlotState slot={slot} referee={referee} /> : <span className="room-player-status">Odada değil</span>}
      {slot?.team ? (
        <IconButton
          icon="swap"
          className={wrongTeam ? "room-player-fix" : undefined}
          label={`${player.username} oyuncusunu ${slot.team === "red" ? "mavi" : "kırmızı"} takıma al`}
          disabled={!inChannel}
          onClick={() => actions.moveToTeam(slot, slot.team === "red" ? "blue" : "red")}
        />
      ) : (
        <IconButton
          icon="userPlus"
          label={`${player.username} oyuncusunu odaya davet et`}
          disabled={!inChannel || slot !== undefined || lobby.closed}
          onClick={() => actions.invite(player.osu_id)}
        />
      )}
    </li>
  );
}

function StrangerRow({ slot, referee }: { slot: RoomSlot; referee: RefereeRoomState }) {
  return (
    <li className={`room-player is-${slot.status} is-stranger`}>
      <span className="room-player-slot">{slot.slot}</span>
      <span className="room-player-text">
        <span className="room-player-name">
          <a href={`https://osu.ppy.sh/users/${slot.userId ?? ""}`} target="_blank" rel="noopener noreferrer">
            {slot.username}
          </a>
          {slot.host && <span className="badge">Host</span>}
        </span>
        <span className="room-player-meta">
          {slot.team ? (slot.team === "red" ? "Kırmızı takımda" : "Mavi takımda") : "Odada"}
        </span>
      </span>
      <SlotState slot={slot} referee={referee} />
    </li>
  );
}

/**
 * Where a player in the room stands: their mods (read from !mp settings as each map starts; with
 * freemod off everyone plays the room's, so nothing per player) and whether they're ready, or
 * their score once they've finished the map being played.
 */
function SlotState({ slot, referee }: { slot: RoomSlot; referee: RefereeRoomState }) {
  const { lobby } = referee;
  const score = lobby.current.find((s) => sameName(s.username, slot.username));
  const freemod = isFreemod(lobby.mods);
  let mods: { text: string; plain: boolean; title?: string } | null = null;
  if (slot.mods) mods = { text: splitMods(slot.mods).join(""), plain: false };
  else if (freemod && !slot.modsKnown) mods = { text: "?", plain: true, title: "Modları henüz okunmadı; Odayı yenile ile okunur" };
  else if (freemod) mods = { text: "NM", plain: true };

  return (
    <>
      <span className={`room-player-mods${mods?.plain ? " is-plain" : ""}`} title={mods?.title}>
        {mods?.text}
      </span>
      {lobby.playing ? (
        score ? (
          <span className={`room-player-score${score.passed ? "" : " is-failed"}`}>
            {score.score.toLocaleString("tr-TR")}
            {!score.passed && <span className="sr-only"> (geçemedi)</span>}
          </span>
        ) : (
          <span className="room-player-status is-playing">Oynuyor</span>
        )
      ) : (
        <span className="room-player-status">{STATUS_LABELS[slot.status]}</span>
      )}
    </>
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
