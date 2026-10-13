import { usePlayer } from "../player/PlayerContext";
import { IconButton } from "./IconButton";
import "./MiniPlayer.css";

export function MiniPlayer() {
  const { track, volume, setVolume, stop } = usePlayer();
  if (!track) return null;

  return (
    <section className="mini-player" aria-label="Song preview">
      <IconButton icon="stop" label="Stop preview" variant="round" active onClick={stop} />
      <div className="mini-player-info">
        <div className="mini-player-title">{track.title}</div>
        {track.subtitle && <div className="mini-player-subtitle">{track.subtitle}</div>}
      </div>
      <input
        className="mini-player-volume"
        type="range"
        min="0"
        max="1"
        step="0.01"
        value={volume}
        onChange={(e) => setVolume(parseFloat(e.target.value))}
        aria-label="Volume"
      />
    </section>
  );
}
