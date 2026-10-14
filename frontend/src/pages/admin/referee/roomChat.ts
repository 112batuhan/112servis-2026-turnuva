import type { RoomChatMessage } from "../../../api/types";
import type { ChatLine } from "../../../irc/IrcContext";

// A room's chat on the referee page is what the database saved (the backend relay
// saves every room line as it passes) followed by what this browser has seen since it
// last asked for that. The two overlap only around the moment of asking.

/** The saved lines of one room, plus where the live conversation stood when they were asked for. */
export interface SavedChat {
  lines: ChatLine[];
  /** The live conversation's last line id when the history was requested. */
  marker: number;
  /** When the history arrived, ms. */
  fetchedAt: number;
}

// Live lines this soon after the history arrived may already be in it.
const OVERLAP_MS = 3000;
// How far back in the history to look for such a duplicate.
const OVERLAP_LINES = 50;

/** Saved lines get negative ids, so they never collide with live ones (which also keeps them out of unread counts). */
export function toChatLine(m: RoomChatMessage): ChatLine {
  return { id: -m.id, at: Date.parse(m.sent_at), from: m.sender, text: m.text, action: m.action, notice: false };
}

/** The saved history followed by the live lines it doesn't have yet. */
export function mergeChat(saved: SavedChat | undefined, live: ChatLine[]): ChatLine[] {
  if (!saved) return live;
  const fresh = live.filter((l) => l.id > saved.marker);
  const tail = saved.lines.slice(-OVERLAP_LINES);
  const matched = new Set<number>();
  const extra = fresh.filter((l) => {
    if (l.notice || l.at > saved.fetchedAt + OVERLAP_MS) return true;
    const i = tail.findIndex((h, j) => !matched.has(j) && h.from === l.from && h.text === l.text);
    if (i === -1) return true;
    matched.add(i);
    return false;
  });
  return [...saved.lines, ...extra];
}
