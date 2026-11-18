import type { RoomChatMessage } from "../api/types";

// A room's chat is what its bot saved (the database), loaded with the page, followed by every line
// the bot saves after that, streamed over the live socket. Both are the same rows, so a line is
// told apart by its id.

/** One line of a room's chat as the pages show it. */
export interface ChatLine {
  id: number;
  /** When it was said, ms. */
  at: number;
  from: string;
  text: string;
  action: boolean;
  /** Bancho refusing a command, rather than someone talking. */
  notice: boolean;
}

/** A saved line, marked when it arrived over the live socket rather than with a load. */
export type StoredLine = RoomChatMessage & { streamed?: boolean };

// Each saved line's ChatLine, made once: a page that rebuilds its rooms' chats (every room reading,
// every new line) hands its rows the same objects, so the rows already drawn aren't drawn again.
const chatLines = new WeakMap<StoredLine, ChatLine>();

/**
 * Loaded lines get negative ids and streamed ones positive, so only what arrived while the page
 * was open counts as unread.
 */
export function toChatLine(m: StoredLine): ChatLine {
  const known = chatLines.get(m);
  if (known) return known;
  const line = {
    id: m.streamed ? m.id : -m.id,
    at: Date.parse(m.sent_at),
    from: m.sender,
    text: m.text,
    action: m.action,
    notice: false,
  };
  chatLines.set(m, line);
  return line;
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
  return lines.slice(-50).some((m) => m.id === message.id)
    ? lines
    : [...lines, { ...message, streamed: true }];
}
