import { useEffect, useState } from "react";
import { fetchQualifierPlaces } from "../api/stats";
import type { QualifierPlace } from "../api/types";

type Places = Record<string, QualifierPlace>;

// Read once for every popover on the page, and again after a while: the places only move while
// lobbies are being played.
const MAX_AGE_MS = 60_000;
let cached: { at: number; places: Places } | null = null;
let loading: Promise<Places> | null = null;

/** Starts reading the places (or reuses a recent read); a failed read counts as none. */
export function loadQualifierPlaces(): Promise<Places> {
  if (cached && Date.now() - cached.at < MAX_AGE_MS) return Promise.resolve(cached.places);
  loading ??= fetchQualifierPlaces()
    .catch(() => ({}))
    .then((places) => {
      cached = { at: Date.now(), places };
      loading = null;
      return places;
    });
  return loading;
}

/**
 * Every team's qualifier place while `enabled` (e.g. a popover is open), or undefined until
 * they're read, so a caller can wait for them before measuring itself.
 */
export function useQualifierPlaces(enabled: boolean): Places | undefined {
  const fresh = cached && Date.now() - cached.at < MAX_AGE_MS ? cached.places : undefined;
  const [places, setPlaces] = useState<Places | undefined>(fresh);
  useEffect(() => {
    if (!enabled) return;
    let live = true;
    void loadQualifierPlaces().then((p) => live && setPlaces(p));
    return () => {
      live = false;
    };
  }, [enabled]);
  return enabled ? places : undefined;
}
