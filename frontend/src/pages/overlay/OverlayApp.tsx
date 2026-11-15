import { useEffect, useState } from "react";
import { Navigate, Route, Routes } from "react-router-dom";
import { ControlDock } from "./ControlDock";
import type { PublicBracket } from "../../api/types";
import type { Camera } from "./bracketBoard";
import { BracketPage } from "./BracketPage";
import { overlayParams, useOverlayControl, type ScreenPage } from "./control";
import { GameplayOverlay } from "./GameplayOverlay";
import { MappoolOverlay } from "./MappoolOverlay";
import { SchedulePage } from "./SchedulePage";
import { TeamsPage } from "./TeamsPage";
import { IPC, useTosu, type TosuTourney } from "./tosu";
import { useBroadcastMatch, type BroadcastMatch } from "./useBroadcastMatch";
import { useHeld, useOverlayBracket, useQualifierSeeds } from "./useOverlayData";
import { WinnerPage } from "./WinnerPage";

// /overlay/*: the broadcast pages, outside the site's shell (no top bar, no player).
// /overlay/stream is the one OBS browser source (1920×1080, transparent page, over the tournament
// client): it shows whichever page the dock picks: gameplay, mappool, the teams before the match,
// the winner after it, the stage's schedule, or the bracket.
// /overlay/control is the streamer's dock that drives it.

const SERIF_FONT =
  "https://fonts.googleapis.com/css2?family=Noto+Serif+Display:wdth,wght@62.5..100,400..800&display=swap";

export default function OverlayApp() {
  return (
    <Routes>
      <Route path="stream" element={<StreamOverlay />} />
      <Route path="control" element={<ControlDock />} />
      <Route path="*" element={<Navigate to="/overlay/control" replace />} />
    </Routes>
  );
}

/** Makes the page transparent and loads the designs' serif while the overlay is open. */
function useOverlayDocument() {
  useEffect(() => {
    const root = document.documentElement;
    root.classList.add("is-overlay");
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = SERIF_FONT;
    document.head.appendChild(link);
    return () => {
      root.classList.remove("is-overlay");
      link.remove();
    };
  }, []);
}

/** Everything a page may draw. */
interface PageData {
  view: BroadcastMatch | null;
  bracket: PublicBracket | null;
  seeds: ReadonlyMap<string, number>;
  camera: Camera | null;
  matchId: string | null;
}

function StreamOverlay() {
  useOverlayDocument();
  const { state } = useOverlayControl();
  const tosu = useTosu(overlayParams().tosu);
  const bracket = useOverlayBracket();
  const seeds = useQualifierSeeds();
  const shared = { tosu, bracket, seeds, camera: state.camera, page: state.page };
  // No match on stream: the pages that need one show the frame's ground alone.
  if (!state.matchId) return <Shown {...shared} view={null} matchId={null} />;
  return <MatchOverlay key={state.matchId} {...shared} matchId={state.matchId} />;
}

interface SharedProps {
  page: ScreenPage;
  tosu: TosuTourney;
  bracket: PublicBracket | null;
  seeds: ReadonlyMap<string, number>;
  camera: Camera | null;
}

function MatchOverlay(props: SharedProps & { matchId: string }) {
  const view = useBroadcastMatch(props.matchId, props.tosu);
  return <Shown {...props} view={view} />;
}

/**
 * The bracket is held back with the match while the tournament clients play a map: its scores and
 * results are the site's, which run ahead of what viewers have seen.
 */
function Shown({
  page,
  tosu,
  bracket,
  view,
  ...rest
}: SharedProps & { view: BroadcastMatch | null; matchId: string | null }) {
  const holding = view ? view.holding : tosu.ipcState === IPC.playing;
  const heldBracket = useHeld(bracket, holding);
  return <Screens page={page} data={{ view, bracket: heldBracket, ...rest }} />;
}

// How long one page takes to give way to the other.
const CROSSFADE_MS = 600;

interface Layer {
  page: ScreenPage;
  leaving: boolean;
}

/**
 * The page on screen, crossfading into the next when it changes: both are drawn from the same
 * match, so the incoming one is ready at once.
 */
function Screens({ page, data }: { page: ScreenPage; data: PageData }) {
  const [layers, setLayers] = useState<Layer[]>([{ page, leaving: false }]);
  useEffect(() => {
    setLayers((prev) =>
      prev.some((l) => l.page === page && !l.leaving)
        ? prev
        : [
            ...prev.filter((l) => l.page !== page).map((l) => ({ ...l, leaving: true })),
            { page, leaving: false },
          ],
    );
    const timer = window.setTimeout(
      () => setLayers((prev) => prev.filter((l) => !l.leaving)),
      CROSSFADE_MS,
    );
    return () => window.clearTimeout(timer);
  }, [page]);
  return (
    <>
      {layers.map((l) => (
        <div key={l.page} className={`ov-layer ${l.leaving ? "is-leaving" : ""}`}>
          <PageView page={l.page} data={data} />
        </div>
      ))}
    </>
  );
}

function PageView({ page, data }: { page: ScreenPage; data: PageData }) {
  switch (page) {
    case "gameplay":
      return <GameplayOverlay view={data.view} />;
    case "mappool":
      return <MappoolOverlay view={data.view} />;
    case "teams":
      return <TeamsPage view={data.view} bracket={data.bracket} seeds={data.seeds} />;
    case "winner":
      return <WinnerPage view={data.view} bracket={data.bracket} />;
    case "schedule":
      return <SchedulePage bracket={data.bracket} matchId={data.matchId} />;
    case "bracket":
      return <BracketPage bracket={data.bracket} camera={data.camera} matchId={data.matchId} />;
  }
}
