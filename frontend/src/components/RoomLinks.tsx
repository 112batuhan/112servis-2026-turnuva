import type { MatchRoom } from "../api/types";

interface Props {
  rooms: MatchRoom[];
  className?: string;
}

/**
 * A match's osu! multiplayer rooms as a compact link: "Maç linki", or "Maç linki: 1 · 2"
 * when a broken room had to be replaced. Renders nothing without a room.
 */
export function RoomLinks({ rooms, className }: Props) {
  if (rooms.length === 0) return null;
  if (rooms.length === 1) {
    return (
      <a className={className} href={rooms[0].url} target="_blank" rel="noopener noreferrer">
        Maç linki
      </a>
    );
  }
  return (
    <span className={className}>
      Maç linki:{" "}
      {rooms.map((room, i) => (
        <span key={room.mp_id}>
          {i > 0 && " · "}
          <a href={room.url} target="_blank" rel="noopener noreferrer" title={`${i + 1}. oda`}>
            {i + 1}
          </a>
        </span>
      ))}
    </span>
  );
}
