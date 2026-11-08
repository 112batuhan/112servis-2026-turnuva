import { useMemo, type CSSProperties } from "react";
import type { PublicMatch } from "../../api/types";
import { BANCHO_BOT, isRoomNoise, sameName } from "../../lib/bancho";
import { poolEntries, type PoolEntry } from "../admin/referee/pool";
import type { PublicRoomChat } from "../match/useMatchPage";
import {
  categoryName,
  MapPill,
  mapMarks,
  OverlayStage,
  SIDE_CLASS,
  StageTitle,
  StarPips,
  TeamBanner,
  TrophyGlow,
  type MapMark,
  type Side,
} from "./parts";
import type { BroadcastMatch } from "./useBroadcastMatch";
import "./MappoolOverlay.css";

// The overlay between maps (design/tournament-client-mappool.png): the whole pool by category,
// each map marked as it's banned or picked, the teams with their stars at the top, and the room's
// chat in the well at the bottom left.

/** The pool's rows: one per category, in the pool's order. */
function poolRows(match: PublicMatch, entries: PoolEntry[]): PoolEntry[][] {
  const categoryOf = new Map(match.slots.map((s) => [s.id, s.category_id]));
  const rows = new Map<string, PoolEntry[]>();
  for (const entry of entries) {
    const category = categoryOf.get(entry.map.slot_id ?? "") ?? "";
    rows.set(category, [...(rows.get(category) ?? []), entry]);
  }
  return [...rows.values()];
}

// The designed grid: 248×86 pills 265px apart, rows 133px apart from y 153; the last row's pills
// end above y 905, or above the chat well (y 859) when the row reaches over it.
const PILL_W = 248;
const PILL_H = 86;
const PILL_GAP = 17;
const ROW_PITCH = 133;
const GRID_TOP = 153;
const MAX_WIDTH = 1700;

function gridScale(rows: PoolEntry[][]): number {
  if (rows.length === 0) return 1;
  const widest = Math.max(...rows.map((r) => r.length));
  const byWidth = MAX_WIDTH / (widest * (PILL_W + PILL_GAP) - PILL_GAP);
  const last = rows[rows.length - 1].length;
  const reachesWell = (last * (PILL_W + PILL_GAP) - PILL_GAP) / 2 > 960 - 760;
  const bottom = reachesWell ? 845 : 905;
  const byHeight = rows.length > 1 ? (bottom - PILL_H - GRID_TOP) / (rows.length - 1) / ROW_PITCH : 1;
  return Math.min(1, byWidth, byHeight);
}

export function MappoolOverlay({ view }: { view: BroadcastMatch | null }) {
  const match = view?.page.match ?? null;
  const tally = view?.page.tally ?? null;
  const team = (side: Side) => {
    const id = side === 1 ? match?.team1_id : match?.team2_id;
    return match?.teams.find((t) => t.id === id) ?? null;
  };
  const marks = useMemo(() => (match ? mapMarks(match, tally) : new Map<string, MapMark>()), [match, tally]);
  const rows = useMemo(() => (match ? poolRows(match, poolEntries(match)) : []), [match]);
  const scale = gridScale(rows);
  const currentId = view?.page.currentEntry?.map.id ?? null;
  const rooms = view?.page.roomChats ?? [];
  const chat = rooms.length > 0 ? rooms[rooms.length - 1] : null;

  return (
    <OverlayStage className="mp">
      {([1, 2] as const).map((side) => (
        <div key={side} className={`mp-team ${SIDE_CLASS[side]}`}>
          <TeamBanner team={team(side)} side={side} />
          <h2 className="ov-team-name mp-team-name">{team(side)?.name ?? ""}</h2>
          <StarPips wins={(side === 1 ? tally?.team1 : tally?.team2) ?? 0} bestOf={match?.stage?.best_of ?? null} side={side} />
        </div>
      ))}
      <StageTitle match={match} />
      <TrophyGlow className="mp-trophy" />
      <div className="mp-grid" style={{ "--mp-scale": scale } as CSSProperties}>
        {rows.map((row, i) => (
          <div key={i} className="mp-row">
            {row.map((entry) => (
              <MapPill
                key={entry.map.id}
                entry={entry}
                mark={marks.get(entry.map.id)}
                tag={categoryName(entry)}
                current={entry.map.id === currentId}
                title
              />
            ))}
          </div>
        ))}
      </div>
      <ChatWell match={match} room={chat} />
    </OverlayStage>
  );
}

// The well holds about seven lines; a few more are kept so the fade at its top has text under it.
const CHAT_LINES = 14;

/**
 * The room's chat as it happens, in the black well: the room's housekeeping left out (as the
 * match page's "Sadeleştir" does), each player's name in their team's colour and the referee's in
 * the info colour, the newest line at the bottom.
 */
function ChatWell({ match, room }: { match: PublicMatch | null; room: PublicRoomChat | null }) {
  const lines = useMemo(
    () => (room?.lines ?? []).filter((l) => !isRoomNoise(l.from, l.text)).slice(-CHAT_LINES),
    [room],
  );
  const sideOf = (name: string): Side | null => {
    if (!match) return null;
    for (const t of match.teams) {
      if (t.players.some((p) => sameName(p.username, name))) {
        return t.id === match.team1_id ? 1 : t.id === match.team2_id ? 2 : null;
      }
    }
    return null;
  };
  const roleOf = (name: string) => {
    const side = sideOf(name);
    if (side) return SIDE_CLASS[side];
    if (match?.referee && sameName(match.referee.username, name)) return "is-referee";
    return name === BANCHO_BOT ? "is-bot" : "";
  };
  return (
    <section className="mp-chat" aria-label="Oda sohbeti">
      <div className="mp-chat-log">
        {lines.map((l) => (
          <p key={l.id} className={`mp-chat-line ${l.action ? "is-action" : ""}`}>
            <span className={`mp-chat-from ${roleOf(l.from)}`}>{l.from}</span>
            <span className="mp-chat-text">{l.action ? ` ${l.text}` : `: ${l.text}`}</span>
          </p>
        ))}
      </div>
    </section>
  );
}
