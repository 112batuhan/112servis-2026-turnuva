import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";
import { errorMessage } from "../../api/client";
import { fetchPublicBracket } from "../../api/bracket";
import type { BracketMatch, PublicBracket } from "../../api/types";
import { Icon } from "../../components/Icon";
import { LiveMark } from "../../components/LiveMark";
import { isLive, matchCode, scoreHalves } from "../../lib/bracket";
import { formatShortMatchTime } from "../../lib/format";
import { useLiveRefresh } from "../../lib/useLiveRefresh";
import { readJson, readString, writeJson, writeString } from "../../lib/storage";
import { useLiveBracket } from "../../live/LiveContext";
import {
  boardBounds,
  fitCamera,
  focusCamera,
  panCamera,
  zoomCamera,
  ZOOM_STEP,
  type Bounds,
  type Camera,
} from "./bracketBoard";
import { APP, broadcast, completeState, isControlMessage, type OverlayState, type ScreenPage } from "./control";
import { DEFAULT_OBS_URL, SUB_GENERAL, useObs, type ObsStatus } from "./obs";
import { DEFAULT_TOSU_ADDR, IPC, useTosu, type TosuReport } from "./tosu";
import "./ControlDock.css";

// The streamer's control panel, opened inside OBS as a custom browser dock (Docks → Custom
// Browser Docks). It owns what the overlay shows (the match, the page /overlay/stream draws and
// where the bracket page looks), keeps it in this browser's storage so OBS restarts with it, and
// sends it over obs-websocket. It reads tosu itself to switch pages on its own: to gameplay as a
// map starts, to the mappool a while after every client reached the results screen, and to the
// winner once the match is concluded (after the results, if the clients are still playing the last
// map). Scenes are left to OBS's own controls.

const OBS_URL_KEY = "overlay-dock-obs-url";
const OBS_PASSWORD_KEY = "overlay-dock-obs-password";
const TOSU_KEY = "overlay-dock-tosu";
const STATE_KEY = "overlay-dock-state";
const AUTO_KEY = "overlay-dock-auto";

// How many finished matches stay in the list, newest first.
const RECENT_LIMIT = 6;
// How long the results stay up before the mappool (or the winner) takes over.
const RESULTS_FOR_MS = 20_000;

/** Which page switches the dock makes on its own. */
interface AutoSwitch {
  /** To the gameplay page as the clients start a map. */
  toGameplay: boolean;
  /** To the mappool RESULTS_FOR_MS after every client reached the results screen. */
  toMappool: boolean;
  /** To the winner once the match on stream is concluded. */
  toWinner: boolean;
}

/** A switch the dock will make by itself when the time comes. */
interface Pending {
  page: ScreenPage;
  at: number;
}

const OBS_LABEL: Record<ObsStatus, string> = {
  connecting: "OBS'e bağlanıyor",
  connected: "OBS bağlı",
  auth_failed: "OBS şifreyi kabul etmedi",
  offline: "OBS'e ulaşılamıyor",
};

const PAGE_NAME: Record<ScreenPage, string> = {
  teams: "Takımlar",
  mappool: "Mappool",
  gameplay: "Oyun",
  winner: "Kazanan",
  schedule: "Maçlar",
  bracket: "Bracket",
};

/** The pages between matches, then the ones in the order a match goes through them. */
const PAGE_ORDER: ScreenPage[] = ["schedule", "bracket", "teams", "mappool", "gameplay", "winner"];

/** "Mappool'a", "Kazanan sayfasına": where a pending switch goes. */
const PENDING_TO: Partial<Record<ScreenPage, string>> = { mappool: "Mappool'a", winner: "Kazanan sayfasına" };

export function ControlDock() {
  const [obsUrl, setObsUrl] = useState(() => readString(OBS_URL_KEY) || DEFAULT_OBS_URL);
  const [obsPassword, setObsPassword] = useState(() => readString(OBS_PASSWORD_KEY) ?? "");
  const [tosuAddr, setTosuAddr] = useState(() => readString(TOSU_KEY) || DEFAULT_TOSU_ADDR);
  const [state, setState] = useState<OverlayState>(() => completeState(readJson<Partial<OverlayState>>(STATE_KEY, {})));
  useEffect(() => writeJson(STATE_KEY, state), [state]);
  const [auto, setAuto] = useState<AutoSwitch>(() => ({
    toGameplay: true,
    toMappool: true,
    toWinner: true,
    ...readJson<Partial<AutoSwitch>>(AUTO_KEY, {}),
  }));
  useEffect(() => writeJson(AUTO_KEY, auto), [auto]);

  const [obsError, setObsError] = useState<string | null>(null);

  const stateRef = useRef(state);
  stateRef.current = state;
  const obs = useObs(obsUrl, obsPassword, SUB_GENERAL, () => {
    // On every (re)connect: hand the overlay the state.
    broadcast(obs.request, { app: APP, kind: "state", state: stateRef.current }).catch(() => {});
  });
  const { request, onEvent, status } = obs;

  useEffect(
    () =>
      onEvent((type, data) => {
        if (type !== "CustomEvent" || !isControlMessage(data)) return;
        if (data.kind === "hello") {
          // The overlay (re)connected and asks what to show.
          broadcast(request, { app: APP, kind: "state", state: stateRef.current }).catch(() => {});
        } else if (data.kind === "state") {
          // Another dock changed it.
          setState(completeState(data.state));
        }
      }),
    [onEvent, request],
  );

  /** Changes what the overlay shows and tells it at once. */
  const update = useCallback(
    (patch: Partial<OverlayState>) => {
      const next = { ...stateRef.current, ...patch };
      stateRef.current = next;
      setState(next);
      setObsError(null);
      broadcast(request, { app: APP, kind: "state", state: next }).catch((e) => setObsError(errorMessage(e)));
    },
    [request],
  );

  // ---------- switching pages on their own, from the tournament client's state ----------

  const tosu = useTosu(tosuAddr);
  const autoRef = useRef(auto);
  autoRef.current = auto;
  const [pending, setPending] = useState<Pending | null>(null);
  // The match was concluded while the clients were still playing its last map: the winner comes
  // after the results instead of the mappool.
  const winnerNext = useRef(false);
  const lastIpc = useRef(tosu.ipcState);
  useEffect(() => {
    const prev = lastIpc.current;
    lastIpc.current = tosu.ipcState;
    if (prev === tosu.ipcState) return;
    if (tosu.ipcState === IPC.playing) {
      // A map started: the results it would have left up for are moot.
      setPending(null);
      if (autoRef.current.toGameplay) update({ page: "gameplay" });
    } else if (prev === IPC.playing && tosu.ipcState === IPC.ranking) {
      // Every client finished the map; the results stay up a while first.
      const page = winnerNext.current && autoRef.current.toWinner ? "winner" : autoRef.current.toMappool ? "mappool" : null;
      winnerNext.current = false;
      if (page) setPending({ page, at: Date.now() + RESULTS_FOR_MS });
    }
  }, [tosu.ipcState, update]);
  useEffect(() => {
    if (pending === null) return;
    const timer = window.setTimeout(() => {
      setPending(null);
      update({ page: pending.page });
    }, Math.max(0, pending.at - Date.now()));
    return () => window.clearTimeout(timer);
  }, [pending, update]);

  const choosePage = (page: ScreenPage) => {
    setPending(null);
    update({ page });
  };
  const setAutoSwitch = (patch: Partial<AutoSwitch>) => {
    if (patch.toMappool === false && pending?.page === "mappool") setPending(null);
    if (patch.toWinner === false && pending?.page === "winner") setPending(null);
    setAuto((a) => ({ ...a, ...patch }));
  };

  // ---------- the matches ----------

  // The bracket, kept current like /bracket.
  const [bracket, setBracket] = useState<PublicBracket | null>(null);
  const [bracketError, setBracketError] = useState<string | null>(null);
  const loadBracket = useCallback(() => {
    fetchPublicBracket()
      .then((b) => {
        setBracket(b);
        setBracketError(null);
      })
      .catch((e) => setBracketError(errorMessage(e)));
  }, []);
  useEffect(loadBracket, [loadBracket]);
  useLiveBracket(loadBracket, loadBracket);
  useLiveRefresh(bracket?.matches ?? [], loadBracket);

  // The match on stream concluded: the winner page, once the viewers have seen the last map end.
  const concluded = useRef<{ id: string | null; at: string | null }>({ id: null, at: null });
  const selectedNode = bracket?.matches.find((m) => m.id === state.matchId) ?? null;
  useEffect(() => {
    const id = selectedNode?.id ?? null;
    const at = selectedNode?.concluded_at ?? null;
    const prev = concluded.current;
    concluded.current = { id, at };
    if (prev.id !== id) {
      winnerNext.current = false;
      return;
    }
    if (prev.at || !at) {
      // Reopened, or nothing new.
      if (!at) winnerNext.current = false;
      return;
    }
    if (!autoRef.current.toWinner) return;
    if (lastIpc.current === IPC.playing) winnerNext.current = true;
    else setPending((p) => (p ? { ...p, page: "winner" } : null) ?? { page: "winner", at: Date.now() });
  }, [selectedNode?.id, selectedNode?.concluded_at]);

  const teamName = (id: string | null) => bracket?.teams.find((t) => t.id === id)?.name ?? "?";
  const groups = useMemo(() => matchGroups(bracket?.matches ?? []), [bracket]);
  const selected = bracket?.matches.find((m) => m.id === state.matchId) ?? null;

  // ---------- the bracket page's camera ----------

  const bounds = useMemo(() => boardBounds(bracket?.matches ?? []), [bracket]);
  const boundsRef = useRef<Bounds | null>(bounds);
  boundsRef.current = bounds;
  const cameraNow = (): Camera => stateRef.current.camera ?? fitCamera(boundsRef.current);
  const camera = {
    pan: (dx: number, dy: number) => update({ camera: panCamera(cameraNow(), dx, dy, boundsRef.current) }),
    zoom: (factor: number) => update({ camera: zoomCamera(cameraNow(), factor, boundsRef.current) }),
    fit: () => update({ camera: null }),
    focus: () => selectedNode && update({ camera: focusCamera(selectedNode) }),
  };
  const zoomShown = Math.round((state.camera ?? fitCamera(bounds)).zoom * 100);

  const saveConnection = (url: string, password: string, tosuAt: string) => {
    writeString(OBS_URL_KEY, url);
    writeString(OBS_PASSWORD_KEY, password);
    writeString(TOSU_KEY, tosuAt);
    setObsUrl(url);
    setObsPassword(password);
    setTosuAddr(tosuAt);
  };

  return (
    <div className="dock">
      <header className="dock-head">
        <h1>Yayın</h1>
        <div className="dock-statuses">
          <span className={`dock-status is-${status}`}>
            <span className="dock-dot" aria-hidden="true" />
            {OBS_LABEL[status]}
          </span>
          <span className={`dock-status is-tosu-${tosu.status}`}>
            <span className="dock-dot" aria-hidden="true" />
            {TOSU_SHORT[tosu.status]}
          </span>
        </div>
      </header>
      {obsError && (
        <p className="dock-error" role="alert">
          {obsError}
        </p>
      )}

      <section className="dock-section" aria-labelledby="dock-page">
        <h2 id="dock-page">Sayfa</h2>
        <div className="dock-pages" role="group" aria-label="Overlay'in gösterdiği sayfa">
          {PAGE_ORDER.map((page) => (
            <button
              key={page}
              type="button"
              className={`dock-page ${state.page === page ? "is-on" : ""}`}
              aria-pressed={state.page === page}
              onClick={() => choosePage(page)}
            >
              {PAGE_NAME[page]}
            </button>
          ))}
        </div>
        {pending !== null && (
          <PendingSwitch
            to={PENDING_TO[pending.page] ?? PAGE_NAME[pending.page]}
            at={pending.at}
            onNow={() => choosePage(pending.page)}
            onCancel={() => setPending(null)}
          />
        )}
        <div className="dock-autos">
          <label className="dock-switch">
            <input
              type="checkbox"
              role="switch"
              checked={auto.toGameplay}
              onChange={(e) => setAutoSwitch({ toGameplay: e.target.checked })}
            />
            <span>Map başlayınca oyun sayfasına geç</span>
          </label>
          <label className="dock-switch">
            <input
              type="checkbox"
              role="switch"
              checked={auto.toMappool}
              onChange={(e) => setAutoSwitch({ toMappool: e.target.checked })}
            />
            <span>Herkes bitirince {RESULTS_FOR_MS / 1000} sn sonra mappool'a geç</span>
          </label>
          <label className="dock-switch">
            <input
              type="checkbox"
              role="switch"
              checked={auto.toWinner}
              onChange={(e) => setAutoSwitch({ toWinner: e.target.checked })}
            />
            <span>Maç bitince kazanan sayfasına geç</span>
          </label>
          {(auto.toGameplay || auto.toMappool || auto.toWinner) && tosu.status !== "connected" && (
            <p className="dock-hint is-warning">Otomatik geçiş tosu'dan okur: {TOSU_LABEL[tosu.status]}</p>
          )}
        </div>
      </section>

      {state.page === "bracket" && (
        <section className="dock-section" aria-labelledby="dock-camera">
          <h2 id="dock-camera">Bracket kamerası</h2>
          <CameraPad
            zoom={zoomShown}
            onPan={camera.pan}
            onZoom={camera.zoom}
            onFit={camera.fit}
            onFocus={selectedNode ? camera.focus : null}
          />
        </section>
      )}

      <section className="dock-section" aria-labelledby="dock-match">
        <h2 id="dock-match">Maç</h2>
        {selected ? (
          <div className="dock-selected">
            <MatchLine match={selected} teamName={teamName} />
            <button type="button" className="btn btn-secondary btn-sm" onClick={() => update({ matchId: null })}>
              Kaldır
            </button>
          </div>
        ) : (
          <p className="dock-empty">Yayında maç yok: overlay boş zemini gösterir.</p>
        )}
        {bracketError && <p className="dock-error">{bracketError}</p>}
        {groups.map(
          (g) =>
            g.matches.length > 0 && (
              <div key={g.title} className="dock-group">
                <h3>{g.title}</h3>
                <ul className="dock-matches">
                  {g.matches.map((m) => (
                    <li key={m.id}>
                      <button
                        type="button"
                        className={`dock-match ${m.id === state.matchId ? "is-on" : ""}`}
                        aria-pressed={m.id === state.matchId}
                        onClick={() => update({ matchId: m.id })}
                      >
                        <MatchLine match={m} teamName={teamName} />
                      </button>
                    </li>
                  ))}
                </ul>
              </div>
            ),
        )}
      </section>

      <Setup
        obsUrl={obsUrl}
        obsPassword={obsPassword}
        tosuAddr={tosuAddr}
        open={status !== "connected"}
        onSave={saveConnection}
      />
    </div>
  );
}

/** A page coming up after the results, counting down, with a way to go now or stay. */
function PendingSwitch({ to, at, onNow, onCancel }: { to: string; at: number; onNow: () => void; onCancel: () => void }) {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), 250);
    return () => window.clearInterval(timer);
  }, []);
  const left = Math.max(0, Math.ceil((at - now) / 1000));
  return (
    <div className="dock-pending" role="status">
      <span>
        {to} <span className="dock-pending-count">{left}</span> sn
      </span>
      <button type="button" className="btn btn-secondary btn-sm" onClick={onNow}>
        Şimdi geç
      </button>
      <button type="button" className="btn btn-secondary btn-sm" onClick={onCancel}>
        Sonuçlarda kal
      </button>
    </div>
  );
}

// A held button repeats after this long, this often, by these smaller steps, so a held arrow is one
// steady glide on stream (the overlay's spring smooths the steps).
const REPEAT_DELAY_MS = 280;
const REPEAT_EVERY_MS = 90;
const PAN_STEP = 0.25;
const PAN_REPEAT = 0.07;
const ZOOM_REPEAT = 1.06;

/**
 * Moves the bracket page's camera: arrows to pan, plus and minus to zoom (each repeating while held,
 * and the keyboard's arrows, +, - and 0 while the pad has focus), the whole board, and the match on
 * stream.
 */
function CameraPad({
  zoom,
  onPan,
  onZoom,
  onFit,
  onFocus,
}: {
  zoom: number;
  onPan: (dx: number, dy: number) => void;
  onZoom: (factor: number) => void;
  onFit: () => void;
  onFocus: (() => void) | null;
}) {
  const onKey = (e: KeyboardEvent) => {
    const step = e.repeat ? PAN_REPEAT : PAN_STEP;
    const keys: Record<string, () => void> = {
      ArrowLeft: () => onPan(-step, 0),
      ArrowRight: () => onPan(step, 0),
      ArrowUp: () => onPan(0, -step),
      ArrowDown: () => onPan(0, step),
      "+": () => onZoom(e.repeat ? ZOOM_REPEAT : ZOOM_STEP),
      "=": () => onZoom(e.repeat ? ZOOM_REPEAT : ZOOM_STEP),
      "-": () => onZoom(1 / (e.repeat ? ZOOM_REPEAT : ZOOM_STEP)),
      "0": onFit,
    };
    const run = keys[e.key];
    if (!run) return;
    e.preventDefault();
    run();
  };
  return (
    <div className="dock-cam" onKeyDown={onKey}>
      <div className="dock-dpad">
        <HoldButton className="is-up" icon="arrowUp" label="Yukarı" first={() => onPan(0, -PAN_STEP)} again={() => onPan(0, -PAN_REPEAT)} />
        <HoldButton className="is-left" icon="arrowLeft" label="Sola" first={() => onPan(-PAN_STEP, 0)} again={() => onPan(-PAN_REPEAT, 0)} />
        <button type="button" className="dock-pad-btn is-center" onClick={onFit} title="Bracket'in tamamı" aria-label="Bracket'in tamamını göster">
          <Icon name="recenter" size={20} />
        </button>
        <HoldButton className="is-right" icon="arrowRight" label="Sağa" first={() => onPan(PAN_STEP, 0)} again={() => onPan(PAN_REPEAT, 0)} />
        <HoldButton className="is-down" icon="arrowDown" label="Aşağı" first={() => onPan(0, PAN_STEP)} again={() => onPan(0, PAN_REPEAT)} />
      </div>
      <div className="dock-zoom">
        <HoldButton icon="plus" label="Yakınlaştır" first={() => onZoom(ZOOM_STEP)} again={() => onZoom(ZOOM_REPEAT)} />
        <span className="dock-zoom-level" aria-live="polite">
          %{zoom}
        </span>
        <HoldButton icon="minus" label="Uzaklaştır" first={() => onZoom(1 / ZOOM_STEP)} again={() => onZoom(1 / ZOOM_REPEAT)} />
      </div>
      <button type="button" className="btn btn-secondary btn-sm dock-focus" onClick={() => onFocus?.()} disabled={!onFocus}>
        <Icon name="target" size={16} /> Maça odaklan
      </button>
    </div>
  );
}

/** A pad button that acts once when pressed, then again and again while it's held. */
function HoldButton({
  icon,
  label,
  className = "",
  first,
  again,
}: {
  icon: "arrowUp" | "arrowDown" | "arrowLeft" | "arrowRight" | "plus" | "minus";
  label: string;
  className?: string;
  first: () => void;
  again: () => void;
}) {
  const timers = useRef<{ delay?: number; every?: number }>({});
  const [held, setHeld] = useState(false);
  const againRef = useRef(again);
  againRef.current = again;
  const stop = () => {
    window.clearTimeout(timers.current.delay);
    window.clearInterval(timers.current.every);
    setHeld(false);
  };
  useEffect(() => stop, []);
  return (
    <button
      type="button"
      className={`dock-pad-btn ${className} ${held ? "is-held" : ""}`}
      aria-label={label}
      title={label}
      onPointerDown={(e) => {
        if (e.button !== 0) return;
        e.currentTarget.setPointerCapture(e.pointerId);
        first();
        setHeld(true);
        timers.current.delay = window.setTimeout(() => {
          timers.current.every = window.setInterval(() => againRef.current(), REPEAT_EVERY_MS);
        }, REPEAT_DELAY_MS);
      }}
      onPointerUp={stop}
      onPointerCancel={stop}
      onLostPointerCapture={stop}
      // The keyboard's Enter and Space (a click with no pointer behind it).
      onClick={(e) => {
        if (e.detail === 0) first();
      }}
    >
      <Icon name={icon} size={20} />
    </button>
  );
}

interface Group {
  title: string;
  matches: BracketMatch[];
}

/** The matches worth putting on stream: live, then upcoming by time, then the latest finished. */
function matchGroups(matches: BracketMatch[]): Group[] {
  const playable = matches.filter((m) => m.team1_id && m.team2_id);
  const live = playable.filter((m) => isLive(m));
  const upcoming = playable
    .filter((m) => !m.concluded_at && !isLive(m))
    .sort((a, b) => (a.starts_at ?? "9").localeCompare(b.starts_at ?? "9"));
  const done = playable
    .filter((m) => m.concluded_at)
    .sort((a, b) => (b.concluded_at ?? "").localeCompare(a.concluded_at ?? ""))
    .slice(0, RECENT_LIMIT);
  return [
    { title: "Canlı", matches: live },
    { title: "Sıradaki", matches: upcoming },
    { title: "Biten", matches: done },
  ];
}

/** A match in one line: its code, the teams and the time, the live mark or the score. */
function MatchLine({ match, teamName }: { match: BracketMatch; teamName: (id: string | null) => string }) {
  const score = scoreHalves(match);
  return (
    <span className="dock-line">
      <span className={`dock-code is-${match.side}`}>{matchCode(match)}</span>
      <span className="dock-teams">
        <span className="is-side-1">{teamName(match.team1_id)}</span>
        <span className="dock-vs">{score ? `${score[0]}–${score[1]}` : "vs"}</span>
        <span className="is-side-2">{teamName(match.team2_id)}</span>
      </span>
      <span className="dock-when">
        {isLive(match) ? <LiveMark compact /> : match.starts_at ? formatShortMatchTime(match.starts_at) : "Zaman yok"}
      </span>
    </span>
  );
}

const TOSU_SHORT: Record<TosuReport, string> = {
  connected: "tosu bağlı",
  connecting: "tosu'ya bağlanıyor",
  unreachable: "tosu'ya ulaşılamıyor",
  no_tourney: "Turnuva istemcisi yok",
};

const TOSU_LABEL: Record<TosuReport, string> = {
  connected: "tosu bağlı",
  connecting: "tosu'ya bağlanıyor",
  unreachable: "tosu'ya ulaşılamıyor: çalışıyor mu, Allowed IPs'te sitenin adresi var mı?",
  no_tourney: "tosu bağlı ama turnuva istemcisi açık değil",
};

/** Connecting to OBS and tosu, and the overlay's address; folded away once OBS is connected. */
function Setup({
  obsUrl,
  obsPassword,
  tosuAddr,
  open,
  onSave,
}: {
  obsUrl: string;
  obsPassword: string;
  tosuAddr: string;
  open: boolean;
  onSave: (url: string, password: string, tosu: string) => void;
}) {
  const [url, setUrl] = useState(obsUrl);
  const [password, setPassword] = useState(obsPassword);
  const [tosu, setTosu] = useState(tosuAddr);
  const [copied, setCopied] = useState(false);

  const params = new URLSearchParams();
  if (obsUrl !== DEFAULT_OBS_URL) params.set("obs", obsUrl);
  if (obsPassword) params.set("pw", obsPassword);
  if (tosuAddr !== DEFAULT_TOSU_ADDR) params.set("tosu", tosuAddr);
  const query = params.toString();
  const link = `${window.location.origin}/overlay/stream${query ? `?${query}` : ""}`;
  const copy = () => {
    navigator.clipboard
      ?.writeText(link)
      .then(() => {
        setCopied(true);
        window.setTimeout(() => setCopied(false), 1500);
      })
      .catch(() => {});
  };

  return (
    <details className="dock-section dock-setup" open={open}>
      <summary>
        Kurulum
        <Icon name="chevronDown" size={16} className="dock-setup-chevron" />
      </summary>
      <form
        className="dock-form"
        onSubmit={(e) => {
          e.preventDefault();
          onSave(url.trim() || DEFAULT_OBS_URL, password, tosu.trim() || DEFAULT_TOSU_ADDR);
        }}
      >
        <p className="dock-hint">OBS'te Araçlar → WebSocket Sunucu Ayarları'ndan sunucuyu aç; şifre oradaki.</p>
        <label>
          <span>OBS adresi</span>
          <input className="input input-sm" value={url} onChange={(e) => setUrl(e.target.value)} spellCheck={false} />
        </label>
        <label>
          <span>Şifre</span>
          <input
            className="input input-sm"
            type="password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            autoComplete="off"
          />
        </label>
        <label>
          <span>tosu adresi</span>
          <input className="input input-sm" value={tosu} onChange={(e) => setTosu(e.target.value)} spellCheck={false} />
        </label>
        <button type="submit" className="btn btn-primary btn-sm">
          Kaydet ve bağlan
        </button>
      </form>
      <div className="dock-sources">
        <p className="dock-hint">
          Overlay, turnuva istemcisinin üstüne 1920×1080 bir tarayıcı kaynağı olarak eklenir; sayfa düğmeleri ve otomatik geçiş
          onu değiştirir.
        </p>
        <div className="dock-source">
          <span className="dock-source-name">Overlay</span>
          <code className="dock-source-url">{link}</code>
          <button type="button" className="btn btn-secondary btn-sm" onClick={copy}>
            {copied ? "Kopyalandı" : "Kopyala"}
          </button>
        </div>
        <p className="dock-hint">
          tosu'nun ayarlarında (127.0.0.1:24050 → Settings → Allowed IPs) listeye <code>{window.location.hostname}</code> ekli olmalı.
        </p>
      </div>
    </details>
  );
}
