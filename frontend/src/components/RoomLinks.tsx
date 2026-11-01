import type { MatchRoom } from "../api/types";
import { Icon } from "./Icon";
import "./RoomLinks.css";

interface Props {
  rooms: MatchRoom[];
  className?: string;
}

/**
 * A match's osu! multiplayer rooms as small buttons: "Maç linki", or, when a broken room had to be
 * replaced, one numbered button per room, oldest first. Renders nothing without a room.
 */
export function RoomLinks({ rooms, className }: Props) {
  if (rooms.length === 0) return null;
  if (rooms.length === 1) {
    return (
      <a
        className={`btn btn-secondary btn-sm room-links${className ? ` ${className}` : ""}`}
        href={rooms[0].url}
        target="_blank"
        rel="noopener noreferrer"
        title="osu!'da aç"
      >
        <Icon name="external" size={13} />
        Maç linki
      </a>
    );
  }
  return (
    <span className={`room-links-set${className ? ` ${className}` : ""}`}>
      {rooms.map((room, i) => (
        <a
          key={room.mp_id}
          className="btn btn-secondary btn-sm room-links is-room"
          href={room.url}
          target="_blank"
          rel="noopener noreferrer"
          title={`${i + 1}. oda: osu!'da aç`}
          aria-label={`Maç linki, ${i + 1}. oda`}
        >
          <Icon name="external" size={13} />
          <span className="room-links-number">{i + 1}</span>
        </a>
      ))}
    </span>
  );
}
