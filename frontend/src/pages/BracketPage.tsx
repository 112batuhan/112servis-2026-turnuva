import { useCallback, useEffect, useMemo, useState } from "react";
import { errorMessage } from "../api/client";
import { fetchPublicBracket } from "../api/bracket";
import type { PublicBracket, Team } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { BracketCalendar } from "../components/bracket/BracketCalendar";
import { BracketGrid } from "../components/bracket/BracketGrid";
import { BracketList } from "../components/bracket/BracketList";
import { BracketSearch, useBracketSearch } from "../components/bracket/BracketSearch";
import {
  BracketViewTabs,
  CountdownToggle,
  useBracketView,
  useCountdownMode,
  useLinkedMatch,
} from "../components/bracket/BracketViewTabs";
import { ErrorAlert } from "../components/ErrorAlert";

export function BracketPage() {
  const { user } = useAuth();
  const [data, setData] = useState<PublicBracket | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useBracketView("public");
  const [countdown, toggleCountdown] = useCountdownMode("public");
  // Selecting a match only highlights it and the lines into and out of it.
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
  // The match a team's match history linked to, which the grid centres on.
  const [gridFocus, setGridFocus] = useState<{ id: string } | null>(null);
  useLinkedMatch((id) => {
    setSelectedId(id);
    setCalendarFocus(null);
    setView("grid");
    setGridFocus({ id });
  });

  const reload = useCallback(async () => setData(await fetchPublicBracket()), []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  const teams = useMemo(() => new Map<string, Team>(data?.teams.map((t) => [t.id, t])), [data]);
  const matches = useMemo(() => data?.matches ?? [], [data]);
  const search = useBracketSearch(matches, teams);
  const shared = {
    matches,
    teams,
    myTeamId: data?.my_team_id,
    viewerId: user?.id,
    selectedId,
    hits: search.hits,
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>Maçlar</h1>
        {data && <span className="badge">{data.matches.length} maç</span>}
        {data && data.matches.length > 0 && (
          <div className="page-header-aside">
            <BracketSearch query={search.query} onChange={search.setQuery} hits={search.hits} />
            {view !== "calendar" && <CountdownToggle on={countdown} onToggle={toggleCountdown} />}
            <BracketViewTabs view={view} onChange={pickView} />
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!data && !error && <p className="status">Yükleniyor…</p>}
      {data?.matches.length === 0 && <p className="status">Bracket henüz hazırlanmadı.</p>}

      {data && data.matches.length > 0 && (
        <>
          {view === "grid" && (
            <BracketGrid
              {...shared}
              centerOn={gridFocus}
              onSelect={setSelectedId}
              onShowInCalendar={showInCalendar}
              countdown={countdown}
            />
          )}
          {view === "list" && <BracketList {...shared} onShowInCalendar={showInCalendar} countdown={countdown} />}
          {view === "calendar" && <BracketCalendar {...shared} focusId={calendarFocus} />}
        </>
      )}
    </div>
  );
}
