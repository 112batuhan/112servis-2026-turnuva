import { createContext, useContext, useEffect, useMemo, useRef, useState } from "react";

// A single app-wide audio player for osu! song previews. osu! serves each beatmapset's
// short preview clip at this URL (the same one the website uses).
const previewUrl = (beatmapsetId) => `https://b.ppy.sh/preview/${beatmapsetId}.mp3`;

function readVolume() {
  try {
    const v = parseFloat(localStorage.getItem("preview-volume"));
    return Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0.7;
  } catch {
    return 0.7;
  }
}

const PlayerContext = createContext(null);

export function PlayerProvider({ children }) {
  const audioRef = useRef(null);
  if (!audioRef.current && typeof Audio !== "undefined") {
    audioRef.current = new Audio();
  }

  // The track currently loaded/playing: { id, title, sub } — null when nothing plays.
  const [track, setTrack] = useState(null);
  const [volume, setVolume] = useState(readVolume);

  // Keep the element volume in sync and remember the choice.
  useEffect(() => {
    if (audioRef.current) audioRef.current.volume = volume;
    try {
      localStorage.setItem("preview-volume", String(volume));
    } catch {
      // ignore write failures (private mode / quota)
    }
  }, [volume]);

  // Clear the player when the clip ends or fails to load (e.g. no preview available).
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

  const stop = () => {
    const audio = audioRef.current;
    if (audio) {
      audio.pause();
      audio.currentTime = 0;
    }
    setTrack(null);
  };

  // Toggle a beatmapset's preview: clicking the one already playing stops it, otherwise
  // switch to and play the new one.
  const toggle = (next) => {
    const audio = audioRef.current;
    if (!audio) return;
    if (track && track.id === next.id) {
      stop();
      return;
    }
    audio.src = previewUrl(next.id);
    audio.volume = volume;
    audio.play().catch(() => {}); // ignore autoplay/network rejections
    setTrack(next);
  };

  const value = useMemo(() => ({ track, volume, setVolume, toggle, stop }), [track, volume]);
  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
}

export function usePlayer() {
  const ctx = useContext(PlayerContext);
  if (!ctx) throw new Error("usePlayer must be used within a PlayerProvider");
  return ctx;
}
