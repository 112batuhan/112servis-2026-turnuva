import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { readString, writeString } from "../lib/storage";

export interface Track {
  beatmapsetId: number;
  title: string;
  subtitle?: string;
}

interface PlayerState {
  track: Track | null;
  volume: number;
  setVolume: (volume: number) => void;
  /** Plays the track, or stops it if it's the one already playing. */
  toggle: (track: Track) => void;
  stop: () => void;
}

const VOLUME_KEY = "preview-volume";

// osu! serves a short preview clip for every beatmapset here.
const previewUrl = (beatmapsetId: number) => `https://b.ppy.sh/preview/${beatmapsetId}.mp3`;

function readVolume(): number {
  const v = parseFloat(readString(VOLUME_KEY) ?? "");
  return Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0.7;
}

const PlayerContext = createContext<PlayerState | null>(null);

export function PlayerProvider({ children }: { children: ReactNode }) {
  const audioRef = useRef<HTMLAudioElement | null>(null);
  if (!audioRef.current && typeof Audio !== "undefined") audioRef.current = new Audio();

  const [track, setTrack] = useState<Track | null>(null);
  const [volume, setVolume] = useState(readVolume);

  useEffect(() => {
    if (audioRef.current) audioRef.current.volume = volume;
    writeString(VOLUME_KEY, String(volume));
  }, [volume]);

  // Clear the player when the clip ends or has no preview.
  useEffect(() => {
    const audio = audioRef.current;
    if (!audio) return;
    const reset = () => setTrack(null);
    audio.addEventListener("ended", reset);
    audio.addEventListener("error", reset);
    return () => {
      audio.removeEventListener("ended", reset);
      audio.removeEventListener("error", reset);
    };
  }, []);

  const stop = useCallback(() => {
    const audio = audioRef.current;
    if (audio) {
      audio.pause();
      audio.currentTime = 0;
    }
    setTrack(null);
  }, []);

  const toggle = useCallback(
    (next: Track) => {
      const audio = audioRef.current;
      if (!audio) return;
      if (track?.beatmapsetId === next.beatmapsetId) {
        stop();
        return;
      }
      audio.src = previewUrl(next.beatmapsetId);
      audio.play().catch(() => {}); // autoplay/network rejections are fine to ignore
      setTrack(next);
    },
    [track, stop],
  );

  const value = useMemo(
    () => ({ track, volume, setVolume, toggle, stop }),
    [track, volume, toggle, stop],
  );
  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
}

export function usePlayer(): PlayerState {
  const ctx = useContext(PlayerContext);
  if (!ctx) throw new Error("usePlayer must be used inside <PlayerProvider>");
  return ctx;
}
