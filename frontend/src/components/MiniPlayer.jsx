import { usePlayer } from "../PlayerContext.jsx";
import "./MiniPlayer.css";

// Fixed bottom-right player shown while a preview is playing: stop button, the track it's
// playing, and a volume slider. Hidden when nothing is playing.
export default function MiniPlayer() {
  const { track, volume, setVolume, stop } = usePlayer();
  if (!track) return null;

  return (
    <div className="mini-player" role="region" aria-label="Song preview player">
      <button className="mini-player-stop" onClick={stop} aria-label="Stop preview">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
          <rect x="5" y="5" width="14" height="14" rx="2" />
        </svg>
      </button>
      <div className="mini-player-info">
        <div className="mini-player-title">{track.title}</div>
        {track.sub && <div className="mini-player-sub">{track.sub}</div>}
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
        title="Volume"
      />
    </div>
  );
}
