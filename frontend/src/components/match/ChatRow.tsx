import { Fragment } from "react";
import { BANCHO_BOT, normalizeName, sameName, type SlotTeam } from "../../lib/bancho";
import { formatClockSeconds } from "../../lib/format";
import type { ChatLine } from "../../lib/roomChat";
import "./ChatRow.css";

const URL_RE = /(https?:\/\/[^\s]+)/g;

export function Linkified({ text }: { text: string }) {
  const parts = text.split(URL_RE);
  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <a key={i} href={part} target="_blank" rel="noopener noreferrer">
            {part}
          </a>
        ) : (
          <Fragment key={i}>{part}</Fragment>
        ),
      )}
    </>
  );
}

interface ChatRowProps {
  line: ChatLine;
  /** Our osu! username, whose lines stand out: our own, or the bot's signed with our name. */
  me: string | null;
  colors: Map<string, SlotTeam | null>;
  /** The match's referee, whose lines stand out on the public page. */
  referee?: string | null;
}

/** One chat line, IRC style: the time, who said it in their team's colour, what they said. */
export function ChatRow({ line, me, colors, referee }: ChatRowProps) {
  const fromMe = me !== null && (sameName(line.from, me) || line.text.startsWith(`[${me}] `));
  const fromReferee = referee != null && normalizeName(line.from) === normalizeName(referee);
  const color = colors.get(normalizeName(line.from));
  const classes = [
    "chat-line",
    line.from === BANCHO_BOT && "is-bot",
    fromMe && "is-me",
    fromReferee && "is-referee",
    line.notice && "is-notice",
    color && `is-team-${color}`,
    colors.has(normalizeName(line.from)) && "is-player",
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={classes}>
      <time className="chat-time">{formatClockSeconds(line.at)}</time>
      <span className="chat-from">{line.action ? `* ${line.from}` : line.from}</span>
      <span className="chat-text">
        <Linkified text={line.text} />
      </span>
    </div>
  );
}
