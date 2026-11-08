import { useEffect, useRef, useState } from "react";
import type { Camera } from "./bracketBoard";
import { DEFAULT_OBS_URL, SUB_GENERAL, useObs } from "./obs";

// What the control dock and the overlay say to each other, as obs-websocket custom events
// (BroadcastCustomEvent → CustomEvent on every client). Every message carries APP so other tools
// on the same OBS are ignored. The dock owns the state: the overlay asks for it with `hello` when
// it connects, and the dock answers with `state` (and sends it again on every change).

export const APP = "112servis-overlay";

/** A page the overlay draws. */
export type ScreenPage = "gameplay" | "mappool" | "teams" | "winner" | "schedule" | "bracket";

export const SCREEN_PAGES: ScreenPage[] = ["gameplay", "mappool", "teams", "winner", "schedule", "bracket"];

/** What the dock has the overlay show. */
export interface OverlayState {
  /** The match on stream (a bracket match's id), or none. */
  matchId: string | null;
  page: ScreenPage;
  /** Where the bracket page looks; null: the whole board. */
  camera: Camera | null;
}

export const INITIAL_STATE: OverlayState = { matchId: null, page: "gameplay", camera: null };

/** A state from an older dock (or its storage) with what it lacks filled in. */
export function completeState(state: Partial<OverlayState> | null | undefined): OverlayState {
  const next = { ...INITIAL_STATE, ...state };
  return SCREEN_PAGES.includes(next.page) ? next : { ...next, page: INITIAL_STATE.page };
}

export type ControlMessage =
  | { app: typeof APP; kind: "hello" }
  | { app: typeof APP; kind: "state"; state: OverlayState };

export function isControlMessage(data: unknown): data is ControlMessage {
  return typeof data === "object" && data !== null && (data as { app?: unknown }).app === APP;
}

/** Sends a control message to every client of the streamer's OBS. */
export function broadcast(request: ReturnType<typeof useObs>["request"], message: ControlMessage): Promise<unknown> {
  return request("BroadcastCustomEvent", { eventData: message });
}

/**
 * The overlay's URL options: `?obs=` (OBS's address), `?pw=` (its password), `?tosu=` (tosu's
 * host:port), and, until the dock says otherwise, `?match=` and `?page=` (testing without the dock).
 */
export function overlayParams() {
  const params = new URLSearchParams(window.location.search);
  return {
    obsUrl: params.get("obs") || DEFAULT_OBS_URL,
    obsPassword: params.get("pw") ?? "",
    matchId: params.get("match"),
    page: (SCREEN_PAGES.find((p) => p === params.get("page")) ?? "gameplay") as ScreenPage,
    tosu: params.get("tosu"),
  };
}

/** The overlay's side of the control channel: the state the dock last sent (or the URL's until it does). */
export function useOverlayControl(): { state: OverlayState } {
  const params = useRef(overlayParams());
  const [state, setState] = useState<OverlayState>({
    ...INITIAL_STATE,
    matchId: params.current.matchId,
    page: params.current.page,
  });

  const obs = useObs(params.current.obsUrl, params.current.obsPassword, SUB_GENERAL, () => {
    // Every (re)connect asks the dock for the state.
    broadcast(obs.request, { app: APP, kind: "hello" }).catch(() => {});
  });

  const { onEvent } = obs;
  useEffect(
    () =>
      onEvent((type, data) => {
        if (type === "CustomEvent" && isControlMessage(data) && data.kind === "state") setState(completeState(data.state));
      }),
    [onEvent],
  );

  return { state };
}
