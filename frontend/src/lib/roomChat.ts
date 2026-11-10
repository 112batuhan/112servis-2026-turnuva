import type { RoomChatMessage } from "../api/types";
import type { ChatLine } from "../irc/IrcContext";

// A room's chat is what its bot saved (the database), loaded with the page, followed by every line
// the bot saves after that, streamed over the live socket. Both are the same rows, so a line is
// told apart by its id.

/** A saved line, marked when it arrived over the live socket rather than with a load. */
export type StoredLine = RoomChatMessage & { streamed?: boolean };

/**
 * Loaded lines get negative ids and streamed ones positive, so only what arrived while the page
 * was open counts as unread, as with live private messages.
 */
export function toChatLine(m: StoredLine): ChatLine {
  return {
    id: m.streamed ? m.id : -m.id,
    at: Date.parse(m.sent_at),
    from: m.sender,
    text: m.text,
    action: m.action,
    notice: false,
  };
}

/** Adds the lines a fetch didn't have yet (they arrived live while it was out) to what it returned. */
export function mergeFetched(fetched: StoredLine[], had: StoredLine[] | undefined): StoredLine[] {
  if (!had || had.length === 0) return fetched;
  const known = new Set(fetched.map((m) => m.id));
  const extra = had.filter((m) => !known.has(m.id));
  return extra.length === 0 ? fetched : [...fetched, ...extra];
}

/** A streamed line added to a room's lines, unless it's already there. */
export function appendLine(lines: StoredLine[], message: RoomChatMessage): StoredLine[] {
  return lines.slice(-50).some((m) => m.id === message.id) ? lines : [...lines, { ...message, streamed: true }];
}
