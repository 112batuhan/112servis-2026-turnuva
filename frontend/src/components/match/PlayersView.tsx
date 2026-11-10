import { useEffect, useRef, useState, type ReactNode } from "react";
import { API_URL } from "../../api/client";
import type { MatchKind, Player, Role, Team } from "../../api/types";
import { ROLE_LABELS } from "../../auth/roles";
import {
  isFreemod,
  isRoomRef,
  roomSize,
  sameName,
  SIDE_COLOR,
  SIDE_SLOTS,
  splitMods,
  type RoomSlot,
  type RoomState,
  type SlotTeam,
} from "../../lib/bancho";
import { formatRank, formatScore } from "../../lib/format";
import type { RosterPlayer } from "../../lib/liveRoom";
import "./PlayersView.css";

const STATUS_LABELS: Record<RoomSlot["status"], string> = {
  ready: "Hazır",
  not_ready: "Hazır değil",
  no_map: "Map yok",
};

/** What the referee can do with the room's slots. */
interface SlotControl {
  /** The room's bot is in the room, which is open: the commands can be sent. */
  enabled: boolean;
  /** Why they can't, while they can't. */
  disabledReason: string;
  /** Moves a player in the room to slot `to` and onto `team`; null leaves either as it is. */
  place: (slot: RoomSlot, to: number | null, team: SlotTeam | null) => void;
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
  /** Says when a bracket player is out of their slot or colour (the referee can fix it). */
  showMisplaced?: boolean;
  /** Makes each slot number open a picker that moves the player (the referee page). */
  slotControl?: SlotControl;
  /** The role a staff member in the room is there as, so they're told apart from strangers. */
  staffRoleOf?: (slot: RoomSlot) => Role | null;
  /** Removes strangers from the room (the referee page). */
  kick?: {
    enabled: boolean;
    disabledReason: string;
    one: (slot: RoomSlot) => void;
    all: () => void;
  };
  /** Why a player's freemod mods show as "?". */
  unreadModsHint?: string;
  /** After the player's rank: how to reach them. */
  renderContact?: (player: P) => ReactNode;
  /** The button at the end of the player's row. */
  renderAction?: (player: P, slot: RoomSlot | undefined) => ReactNode;
}

/**
 * The match's players where the room has them: each team under a rule in its colour (blue then
 * red side by side in a bracket match, every lobby team in a grid), each player pinned to their
 * row whatever slot they're in. Each row leads with the slot the player is in, in the colour of
 * the team they're on in the room, then whether they're ready, their mods and, while a map is
 * played, their score. Whoever is in the room without being in the match is listed after. Shared
 * by the referee page and the public match page, which leaves out the contact and the controls.
 */
export function PlayersView<P extends Player>(props: PlayersViewProps<P>) {
  const { kind, roster, lobby, findRoster, finished } = props;
  const colorOf = (team: Team<P>) => roster.find((r) => r.team.id === team.id)?.color ?? null;
  // A bracket match's blue team (team 1, slots 1 and 2) first.
  const teams =
    kind === "bracket"
      ? [...props.teams].sort((a, b) => Number(colorOf(a) === "red") - Number(colorOf(b) === "red"))
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
      {strangers.length > 0 && <Strangers strangers={strangers} view={props} />}
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
          <PlayerRow
            key={p.id}
            player={p}
            slot={slots[i]}
            color={color}
            home={view.roster.find((r) => r.player.id === p.id)?.home ?? null}
            view={view}
          />
        ))}
      </ul>
    </div>
  );
}

function PlayerRow<P extends Player>({
  player,
  slot,
  color,
  home,
  view,
}: {
  player: P;
  slot: RoomSlot | undefined;
  /** The colour their team plays as; null in a lobby. */
  color: SlotTeam | null;
  /** The slot they belong in; null in a lobby. */
  home: number | null;
  view: PlayersViewProps<P>;
}) {
  const { lobby, showMisplaced, renderContact, renderAction } = view;
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
  const misplaced = Boolean(
    slot && ((home !== null && slot.slot !== home) || (color !== null && slot.team !== null && slot.team !== color)),
  );
  const classes = [
    "room-player",
    slot ? `is-${slot.status}` : "is-away",
    misplaced && showMisplaced && "is-misplaced",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <li className={classes}>
      <SlotNumber
        name={player.username}
        slot={slot}
        home={home}
        color={color}
        misplaced={misplaced && Boolean(showMisplaced)}
        view={view}
      />
      <img className="room-player-avatar" src={player.avatar_url} alt="" loading="lazy" />
      <span className="room-player-text">
        <span className="room-player-name">
          <a href={`https://osu.ppy.sh/users/${player.osu_id}`} target="_blank" rel="noopener noreferrer">
            {player.username}
          </a>
          {slot?.host && <span className="badge">Host</span>}
        </span>
        <span className="room-player-meta">
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
      {renderAction && (renderAction(player, slot) ?? <span />)}
    </li>
  );
}

/**
 * Whoever is in the room without playing in the match: the staff who may be there, named by
 * their role, and anyone else, who shouldn't be. The referee kicks those one by one or all at
 * once (the room's bot also kicks them as they join).
 */
function Strangers<P extends Player>({ strangers, view }: { strangers: RoomSlot[]; view: PlayersViewProps<P> }) {
  const outsiders = strangers.filter((s) => !view.staffRoleOf?.(s));
  const { kick } = view;
  return (
    <div className={`room-players-strangers${outsiders.length > 0 ? " has-outsiders" : ""}`}>
      <h3 className="room-players-team-name">
        <span className="room-players-team-title">Maçın oyuncusu değil</span>
        {kick && outsiders.length > 1 && (
          <button
            type="button"
            className="btn btn-danger btn-sm room-players-kick-all"
            disabled={!kick.enabled}
            title={kick.enabled ? "Maçın oyuncusu, host, hakem ya da yayıncı olmayan herkesi odadan atar" : kick.disabledReason}
            onClick={() => {
              if (window.confirm(`${outsiders.map((s) => s.username).join(", ")} odadan atılsın mı?`)) kick.all();
            }}
          >
            Hepsini at ({outsiders.length})
          </button>
        )}
      </h3>
      <ul className="room-players-list">
        {strangers.map((slot) => (
          <StrangerRow key={slot.slot} slot={slot} role={view.staffRoleOf?.(slot) ?? null} view={view} />
        ))}
      </ul>
    </div>
  );
}

function StrangerRow<P extends Player>({
  slot,
  role,
  view,
}: {
  slot: RoomSlot;
  /** Staff are named by their role and never kicked from here. */
  role: Role | null;
  view: PlayersViewProps<P>;
}) {
  const { kick } = view;
  return (
    <li className={`room-player is-${slot.status} is-stranger${role ? " is-staff" : " is-outsider"}`}>
      <SlotNumber name={slot.username} slot={slot} home={null} color={null} misplaced={false} view={view} />
      <span className="room-player-text">
        <span className="room-player-name">
          <a href={`https://osu.ppy.sh/users/${slot.userId ?? ""}`} target="_blank" rel="noopener noreferrer">
            {slot.username}
          </a>
          {role && <span className="badge badge-info">{STAFF_LABELS[role] ?? ROLE_LABELS[role]}</span>}
          {slot.host && <span className="badge">Host</span>}
        </span>
        <span className="room-player-meta">
          {role ? "Turnuva görevlisi" : <span className="room-player-wrong">Maçta yeri yok</span>}
          {isRoomRef(view.lobby, slot.username) && " · odanın hakemi"}
          {slot.team && ` · ${TEAM_NAMES[slot.team]} takımda`}
        </span>
      </span>
      <SlotState slot={slot} lobby={view.lobby} hint={view.unreadModsHint} />
      {kick &&
        (role ? (
          <span />
        ) : (
          <button
            type="button"
            className="btn btn-danger btn-sm room-player-kick"
            disabled={!kick.enabled}
            title={kick.enabled ? `!mp kick: ${slot.username} odadan atılır` : kick.disabledReason}
            onClick={() => kick.one(slot)}
          >
            Odadan at
          </button>
        ))}
    </li>
  );
}

// A room host badge already says "Host", so the tournament's hosts are named in full.
const STAFF_LABELS: Partial<Record<Role, string>> = { host: "Turnuva hostu" };

const TEAM_NAMES: Record<SlotTeam, string> = { blue: "mavi", red: "kırmızı" };

/** "2. slot, mavi takım" */
function describeSlot(slot: RoomSlot | undefined): string {
  if (!slot) return "odada değil";
  return slot.team ? `${slot.slot}. slot, ${TEAM_NAMES[slot.team]} takım` : `${slot.slot}. slot`;
}

/**
 * The slot a player is in, in the colour of the team they're on in the room (a dash while they're
 * not in it). For the referee it's a button that opens the slot picker.
 */
function SlotNumber<P extends Player>({
  name,
  slot,
  home,
  color,
  misplaced,
  view,
}: {
  name: string;
  slot: RoomSlot | undefined;
  home: number | null;
  color: SlotTeam | null;
  misplaced: boolean;
  view: PlayersViewProps<P>;
}) {
  const [open, setOpen] = useState(false);
  const trigger = useRef<HTMLButtonElement>(null);
  const classes = ["room-player-slot", slot?.team && `is-team-${slot.team}`, misplaced && "is-misplaced"]
    .filter(Boolean)
    .join(" ");
  const figure = slot ? slot.slot : "–";
  // Where they belong, said only when they aren't there: the chip's colour and figure show it.
  const belongs = misplaced && home !== null && color !== null ? `; yeri ${home}. slot, ${TEAM_NAMES[color]}` : "";
  const control = view.slotControl;
  if (!control || !slot) {
    return (
      <span
        className={classes}
        title={slot ? describeSlot(slot) + belongs : home ? `Odada değil; yeri ${home}. slot` : "Odada değil"}
      >
        {figure}
        {belongs && <span className="sr-only">{belongs}</span>}
      </span>
    );
  }
  const close = (refocus = true) => {
    setOpen(false);
    if (refocus) trigger.current?.focus();
  };
  return (
    <span className="room-player-slot-cell">
      <button
        ref={trigger}
        type="button"
        className={`${classes} is-button`}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`${name}: ${describeSlot(slot)}${belongs}. Yerini değiştir`}
        title={control.enabled ? `${describeSlot(slot)}${belongs} · yerini değiştir` : control.disabledReason}
        disabled={!control.enabled}
        onClick={() => setOpen((o) => !o)}
      >
        {figure}
      </button>
      {open && (
        <SlotPicker
          name={name}
          slot={slot}
          home={home}
          color={color}
          lobby={view.lobby}
          teamMode={view.kind === "bracket"}
          size={roomSize(view.lobby, view.kind === "bracket" ? 4 : 0)}
          onPlace={(to, team) => control.place(slot, to, team)}
          onClose={close}
        />
      )}
    </span>
  );
}

/**
 * Where to put a player: every slot of the room (a bracket match's 1-2 marked blue and 3-4 red,
 * each with who holds it), the two colours, and, when the player is out of place, one button that
 * sends them home. A slot someone else holds is swapped. Each choice is sent to the room at once.
 */
function SlotPicker({
  name,
  slot,
  home,
  color,
  lobby,
  teamMode,
  size,
  onPlace,
  onClose,
}: {
  name: string;
  slot: RoomSlot;
  home: number | null;
  color: SlotTeam | null;
  lobby: RoomState;
  teamMode: boolean;
  size: number;
  onPlace: (to: number | null, team: SlotTeam | null) => void;
  /** Closes the picker, focusing its slot number again unless `refocus` is false. */
  onClose: (refocus?: boolean) => void;
}) {
  const ref = useRef<HTMLDivElement>(null);
  // Read through a ref, so the page re-rendering (every chat line) neither refocuses nor rebinds.
  const closeRef = useRef(onClose);
  closeRef.current = onClose;
  useEffect(() => {
    ref.current?.querySelector<HTMLButtonElement>("[aria-pressed='true'], button")?.focus();
    const away = (e: PointerEvent) => {
      // Closed by a click elsewhere, focus stays where the click put it.
      if (!ref.current?.parentElement?.contains(e.target as Node)) closeRef.current(false);
    };
    document.addEventListener("pointerdown", away);
    return () => document.removeEventListener("pointerdown", away);
  }, []);
  const zone = (n: number): SlotTeam | null =>
    !teamMode ? null : SIDE_SLOTS[1].includes(n) ? SIDE_COLOR[1] : SIDE_SLOTS[2].includes(n) ? SIDE_COLOR[2] : null;
  const atHome = (home === null || slot.slot === home) && (color === null || slot.team === color);

  return (
    <div
      ref={ref}
      className="slot-picker"
      role="dialog"
      aria-label={`${name}: yer ve takım`}
      onKeyDown={(e) => {
        if (e.key !== "Escape") return;
        e.stopPropagation();
        onClose();
      }}
    >
      <p className="slot-picker-head">
        <span className="slot-picker-name">{name}</span>
        <span className="slot-picker-now">şu an {describeSlot(slot)}</span>
      </p>
      <div className="slot-picker-slots" role="group" aria-label="Slot">
        {Array.from({ length: size }, (_, i) => i + 1).map((n) => {
          const holder = lobby.slots.find((s) => s.slot === n);
          const mine = n === slot.slot;
          const z = zone(n);
          return (
            <button
              key={n}
              type="button"
              className={["slot-picker-slot", z && `is-zone-${z}`, n === home && "is-home"].filter(Boolean).join(" ")}
              aria-pressed={mine}
              aria-label={`${n}. slot${holder && !mine ? `, ${holder.username} ile yer değiştir` : ""}${n === home ? " (yeri)" : ""}`}
              onClick={() => !mine && onPlace(n, null)}
            >
              <span className={`room-player-slot is-small${holder?.team ? ` is-team-${holder.team}` : holder ? "" : " is-free"}`}>
                {n}
              </span>
              <span className="slot-picker-holder">{mine ? "burada" : (holder?.username ?? "boş")}</span>
              {n === home && <span className="slot-picker-home">yeri</span>}
            </button>
          );
        })}
      </div>
      {teamMode && (
        <div className="slot-picker-teams" role="group" aria-label="Takım">
          {SIDE_ORDER.map((team) => (
            <button
              key={team}
              type="button"
              className={`slot-picker-team is-team-${team}`}
              aria-pressed={slot.team === team}
              onClick={() => slot.team !== team && onPlace(null, team)}
            >
              {TEAM_NAMES[team][0].toLocaleUpperCase("tr-TR") + TEAM_NAMES[team].slice(1)}
            </button>
          ))}
        </div>
      )}
      {!atHome && home !== null && color !== null && (
        <button
          type="button"
          className="btn btn-primary btn-sm slot-picker-home-button"
          onClick={() => {
            onPlace(home, color);
            onClose();
          }}
        >
          Yerine gönder: {home}. slot, {TEAM_NAMES[color]}
        </button>
      )}
    </div>
  );
}

/** The colours in slot order: blue holds 1 and 2, red 3 and 4. */
const SIDE_ORDER: SlotTeam[] = [SIDE_COLOR[1], SIDE_COLOR[2]];

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
            {formatScore(score.score)}
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
