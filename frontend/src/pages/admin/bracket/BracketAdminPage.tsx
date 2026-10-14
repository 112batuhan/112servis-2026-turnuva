import { useEffect, useMemo, useState } from "react";
import { fetchStreamers } from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import { fetchStages } from "../../../api/mappool";
import { fetchReferees } from "../../../api/matches";
import type { BracketSide, Player, Stage } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import { BracketCalendar } from "../../../components/bracket/BracketCalendar";
import { BracketGrid } from "../../../components/bracket/BracketGrid";
import { BracketList } from "../../../components/bracket/BracketList";
import { BracketSearch, useBracketSearch } from "../../../components/bracket/BracketSearch";
import {
  BracketViewTabs,
  CountdownToggle,
  useBracketView,
  useCountdownMode,
} from "../../../components/bracket/BracketViewTabs";
import { RefereeRoomLink } from "../../../components/bracket/RefereeRoomLink";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { firstFreeSpot, incomingSources, SIDE_LABELS } from "../../../lib/bracket";
import { MatchPanel } from "./MatchPanel";
import { RefereeControl, StreamerControl } from "./StaffControls";
import { useBracketEditor, type EditorMatch, type EditorTeam } from "./useBracketEditor";
import "./BracketAdminPage.css";

export function BracketAdminPage() {
  const user = useSignedInUser();
  const isHost = hasRole(user, "host");
  const isReferee = hasRole(user, "referee");
  const editor = useBracketEditor(isReferee);
  const { data, error, setError, busy } = editor;
  const [view, setView] = useBracketView("admin");
  const [countdown, toggleCountdown] = useCountdownMode("admin");
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // The match a clicked time opens the calendar on; picking a view from the tabs clears it.
  const [calendarFocus, setCalendarFocus] = useState<string | null>(null);
  const pickView = (next: typeof view) => {
    setCalendarFocus(null);
    setView(next);
  };
  const showInCalendar = (id: string) => {
    setSelectedId(id);
    setCalendarFocus(id);
    setView("calendar");
  };
  const [newSide, setNewSide] = useState<BracketSide>("winners");
  const [referees, setReferees] = useState<Player[]>([]);
  const [streamers, setStreamers] = useState<Player[]>([]);
  const [stages, setStages] = useState<Stage[]>([]);

  useEffect(() => {
    if (!isHost) return;
    fetchReferees().then(setReferees).catch((e) => setError(errorMessage(e)));
    fetchStreamers().then(setStreamers).catch((e) => setError(errorMessage(e)));
    fetchStages().then(setStages).catch((e) => setError(errorMessage(e)));
  }, [isHost, setError]);

  const matches = useMemo(() => data?.matches ?? [], [data]);
  const teams = useMemo(() => new Map<string, EditorTeam>(data?.teams.map((t) => [t.id, t])), [data]);
  const search = useBracketSearch(matches, teams);
  const sources = useMemo(() => incomingSources<EditorMatch>(matches), [matches]);
  const selected = matches.find((m) => m.id === selectedId) ?? null;

  async function add(x: number, y: number) {
    const id = await editor.add(newSide, x, y);
    if (id) setSelectedId(id);
  }

  const unrefereed = matches.filter((m) => !m.referee && !m.concluded_at).length;
  const unstreamed = matches.filter((m) => !m.streamer && !m.concluded_at).length;
  const shared = {
    matches,
    teams,
    myTeamId: data?.myTeamId,
    viewerId: user.id,
    selectedId,
    hits: search.hits,
  };

  return (
    <section className="panel bracket-admin">
      <div className="page-header">
        <h1>Bracket</h1>
        {data && <span className="badge">{matches.length} maç</span>}
        {unrefereed > 0 && (
          <span className="badge badge-warning" title="Henüz hakemi olmayan maçlar">
            {unrefereed} hakemsiz
          </span>
        )}
        {unstreamed > 0 && (
          <span className="badge" title="Henüz yayıncısı olmayan maçlar">
            {unstreamed} yayıncısız
          </span>
        )}
        {data && (
          <div className="page-header-aside">
            <BracketSearch query={search.query} onChange={search.setQuery} hits={search.hits} />
            {view !== "calendar" && <CountdownToggle on={countdown} onToggle={toggleCountdown} />}
            <BracketViewTabs view={view} onChange={pickView} />
            {isHost && (
              <span className="inline-form">
                <select
                  className="select select-sm"
                  aria-label="Yeni maçın tarafı"
                  value={newSide}
                  onChange={(e) => setNewSide(e.target.value as BracketSide)}
                >
                  {(["winners", "losers"] as const).map((s) => (
                    <option key={s} value={s}>
                      {SIDE_LABELS[s]}
                    </option>
                  ))}
                </select>
                <button
                  type="button"
                  className="btn btn-primary btn-sm"
                  disabled={busy}
                  onClick={() => {
                    const spot = firstFreeSpot(matches);
                    add(spot.x, spot.y);
                  }}
                >
                  Maç ekle
                </button>
              </span>
            )}
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!data && !error && <p className="status">Yükleniyor…</p>}
      {data && isHost && view === "grid" && (
        <p className="bracket-admin-hint">
          Maçları sürükleyerek istediğin yere taşı, ızgaraya oturur. Boş alanı sürükleyerek görünümü kaydır, boş bir
          yere çift tıklayarak oraya{" "}
          {SIDE_LABELS[newSide]} maçı ekle. Bir maçı seçince kazananın ve kaybedenin nereye gideceğini panelden
          ayarlayabilirsin.
        </p>
      )}
      {data && matches.length === 0 && !isHost && <p className="status">Henüz bracket maçı yok.</p>}

      {data && (matches.length > 0 || isHost) && (
        <div className={selected ? "bracket-admin-body has-panel" : "bracket-admin-body"}>
          <div className="bracket-admin-main">
            {view === "grid" && (
              <BracketGrid
                {...shared}
                onSelect={setSelectedId}
                editable={isHost}
                onMove={editor.move}
                onAdd={isHost ? (x, y) => add(x, y) : undefined}
                onShowInCalendar={showInCalendar}
                countdown={countdown}
                showRefereeRoom={isReferee}
              />
            )}
            {view === "list" && (
              <BracketList
                {...shared}
                onSelect={setSelectedId}
                renderReferee={(m) => <RefereeControl match={m} editor={editor} options={referees} discord />}
                renderStreamer={(m) => (
                  <StreamerControl match={m} editor={editor} options={streamers} myTeamId={data.myTeamId} />
                )}
                renderActions={isReferee ? (m) => <RefereeRoomLink matchId={m.id} variant="row" /> : undefined}
                onShowInCalendar={showInCalendar}
                countdown={countdown}
              />
            )}
            {view === "calendar" && (
              <BracketCalendar
                {...shared}
                onSelect={setSelectedId}
                focusId={calendarFocus}
                showRefereeRoom={isReferee}
                showUnscheduled={isHost}
              />
            )}
          </div>
          {selected && (
            <MatchPanel
              match={selected}
              matches={matches}
              teams={teams}
              sources={sources}
              myTeamId={data.myTeamId}
              editor={editor}
              referees={referees}
              streamers={streamers}
              stages={stages}
              onClose={() => setSelectedId(null)}
            />
          )}
        </div>
      )}
    </section>
  );
}
