import { useEffect, useState, type ReactNode } from "react";
import { API_URL, errorMessage } from "../../../api/client";
import { pingPlayer } from "../../../api/discord";
import type { AdminTeamPlayer, MatchKind, Player, Team } from "../../../api/types";
import { DiscordMention } from "../../../components/DiscordMention";
import { IconButton } from "../../../components/IconButton";
import { isFreemod, sameName, splitMods, type RoomSlot, type RoomState, type SlotTeam } from "../../../lib/bancho";
import { formatRank } from "../../../lib/format";
import type { RefereeRoomState, RosterPlayer } from "./useRefereeRoom";
import "./RoomPlayers.css";

const STATUS_LABELS: Record<RoomSlot["status"], string> = {
  ready: "Hazır",
  not_ready: "Hazır değil",
  no_map: "Map yok",
};

/**
 * The referee's view of the room's players: each with their Discord (which pings them in the
 * match channel) and the button to invite them or move them to the other colour.
 */
export function RoomPlayers({ referee }: { referee: RefereeRoomState }) {
  const { room, roster, lobby, findRoster, actions, inChannel, canRun } = referee;
  if (!room) return null;
  return (
    <PlayersView
      kind={room.kind}
      teams={room.teams}
      roster={roster}
      lobby={lobby}
      findRoster={findRoster}
      finished={room.concluded_at !== null}
      showWrongTeam
      unreadModsHint="Modları henüz okunmadı; Odayı yenile ile okunur"
      renderContact={(player) =>
        room.discord_channel && canRun ? (
          <DiscordPing matchId={room.id} player={player} />
        ) : (
          <DiscordMention player={player} fallback="Discord yok" />
        )
      }
      renderAction={(player, slot, wrongTeam) =>
        slot?.team ? (
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
        )
      }
    />
  );
}

interface PlayersViewProps<P extends Player> {
  kind: MatchKind;
  /** A bracket match's two teams, or a lobby's. */
  teams: Team<P>[];
  roster: RosterPlayer<P>[];
  lobby: RoomState;
  findRoster: (slot: Pick<RoomSlot, "username" | "userId">) => RosterPlayer<P> | undefined;
  /**
   * The match is over: the room's state no longer says anything, so the teams are listed as a
   * roster, without slots, ready states, mods, who's in the room or the buttons that act on it.
   */
  finished?: boolean;
  /** Heads each team with its banner, the name set on it (the public match page). */
  banners?: boolean;
  /** Says when a bracket player is on the other team's colour (the referee can fix it). */
  showWrongTeam?: boolean;
  /** Why a player's freemod mods show as "?". */
  unreadModsHint?: string;
  /** After the player's rank: how to reach them. */
  renderContact?: (player: P) => ReactNode;
  /** The button at the end of the player's row. */
  renderAction?: (player: P, slot: RoomSlot | undefined, wrongTeam: boolean) => ReactNode;
}

/**
 * The match's players where the room has them: each team under a rule in its colour (red then
 * blue side by side in a bracket match, every lobby team in a grid), each player with their slot,
 * whether they're ready, their mods and, while a map is played, their score. Whoever is in the
 * room without being in the match is listed after. Shared by the referee page and the public
 * match page, which leaves out the contact and the buttons.
 */
export function PlayersView<P extends Player>(props: PlayersViewProps<P>) {
  const { kind, roster, lobby, findRoster, finished } = props;
  const colorOf = (team: Team<P>) => roster.find((r) => r.team.id === team.id)?.color ?? null;
  // A bracket match's red team first.
  const teams =
    kind === "bracket"
      ? [...props.teams].sort((a, b) => Number(colorOf(a) === "blue") - Number(colorOf(b) === "blue"))
      : props.teams;
  const strangers = finished ? [] : lobby.slots.filter((s) => !findRoster(s));
  const mode = finished ? " is-roster" : props.renderAction ? "" : " is-static";

  return (
    <section className={`room-players${mode}`} aria-label="Oyuncular">
      {teams.length === 0 && <p className="muted">Bu maçta henüz takım yok.</p>}
      <div className={`room-players-teams${kind === "bracket" ? " is-versus" : ""}`}>
        {teams.map((team) => (
          <TeamPlayers key={team.id} team={team} color={colorOf(team)} view={props} />
        ))}
      </div>
      {strangers.length > 0 && (
        <div className="room-players-strangers">
          <h3 className="room-players-team-name">Maçın oyuncusu değil</h3>
          <ul className="room-players-list">
            {strangers.map((slot) => (
              <StrangerRow key={slot.slot} slot={slot} lobby={lobby} hint={props.unreadModsHint} />
            ))}
          </ul>
        </div>
      )}
    </section>
  );
}

function TeamPlayers<P extends Player>({
  team,
  color,
  view,
}: {
  team: Team<P>;
  color: SlotTeam | null;
  view: PlayersViewProps<P>;
}) {
  const { lobby } = view;
  const slots = team.players.map((p) => lobby.slots.find((s) => s.userId === p.osu_id || sameName(s.username, p.username)));
  const inRoom = slots.filter(Boolean).length;
  const name = (
    <h3 className="room-players-team-name">
      <span className="room-players-team-title">{team.name}</span>
      {!view.finished && (
        <span className="room-players-team-count">
          {inRoom}/{team.players.length} odada
        </span>
      )}
    </h3>
  );
  return (
    <div className={`room-players-team${color ? ` is-team-${color}` : ""}`}>
      {view.banners ? (
        // The banner under the side's rule, the name set on its foot as on a team card.
        <div className={`room-players-banner${team.banner_url ? "" : " is-empty"}`}>
          {team.banner_url ? (
            <img src={`${API_URL}${team.banner_url}`} alt="" loading="lazy" />
          ) : (
            <span className="room-players-monogram" aria-hidden="true">
              {Array.from(team.name)[0]?.toLocaleUpperCase("tr-TR")}
            </span>
          )}
          {name}
        </div>
      ) : (
        name
      )}
      <ul className="room-players-list">
        {team.players.map((p, i) => (
          <PlayerRow key={p.id} player={p} slot={slots[i]} color={color} view={view} />
        ))}
      </ul>
    </div>
  );
}

function PlayerRow<P extends Player>({
  player,
  slot,
  color,
  view,
}: {
  player: P;
  slot: RoomSlot | undefined;
  color: SlotTeam | null;
  view: PlayersViewProps<P>;
}) {
  const { lobby, showWrongTeam, renderContact, renderAction } = view;
  const wrongTeam = Boolean(slot && color && slot.team && slot.team !== color);
  if (view.finished) {
    return (
      <li className="room-player">
        <img className="room-player-avatar" src={player.avatar_url} alt="" loading="lazy" />
        <span className="room-player-text">
          <span className="room-player-name">
            <a href={`https://osu.ppy.sh/users/${player.osu_id}`} target="_blank" rel="noopener noreferrer">
              {player.username}
            </a>
          </span>
          <span className="room-player-meta">
            {formatRank(player.global_rank)}
            {renderContact && <> · {renderContact(player)}</>}
          </span>
        </span>
      </li>
    );
  }
  return (
    <li className={`room-player${slot ? ` is-${slot.status}` : " is-away"}${wrongTeam && showWrongTeam ? " is-wrong" : ""}`}>
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
          {wrongTeam && showWrongTeam && <span className="room-player-wrong">Yanlış takımda · </span>}
          {formatRank(player.global_rank)}
          {renderContact && <> · {renderContact(player)}</>}
        </span>
      </span>
      {slot ? (
        <SlotState slot={slot} lobby={lobby} hint={view.unreadModsHint} />
      ) : (
        <>
          <span />
          <span className="room-player-status">Odada değil</span>
        </>
      )}
      {renderAction?.(player, slot, wrongTeam)}
    </li>
  );
}

function StrangerRow({ slot, lobby, hint }: { slot: RoomSlot; lobby: RoomState; hint: string | undefined }) {
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
      <SlotState slot={slot} lobby={lobby} hint={hint} />
    </li>
  );
}

/**
 * Where a player in the room stands: their mods (read from !mp settings as each map starts; with
 * freemod off everyone plays the room's, so nothing per player) and whether they're ready, or
 * their score once they've finished the map being played.
 */
function SlotState({ slot, lobby, hint }: { slot: RoomSlot; lobby: RoomState; hint: string | undefined }) {
  const score = lobby.current.find((s) => sameName(s.username, slot.username));
  const freemod = isFreemod(lobby.mods);
  let mods: { text: string; plain: boolean; title?: string } | null = null;
  if (slot.mods) mods = { text: splitMods(slot.mods).join(""), plain: false };
  else if (freemod && !slot.modsKnown) mods = { text: "?", plain: true, title: hint ?? "Modları henüz okunmadı" };
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
