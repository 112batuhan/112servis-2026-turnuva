import { useCallback, useEffect, useMemo, useState } from "react";
import { errorMessage } from "../../api/client";
import { fetchPublicBracket } from "../../api/bracket";
import type { PublicBracket, Team } from "../../api/types";
import { useAuth } from "../../auth/AuthContext";
import { BracketCalendar } from "../../components/bracket/BracketCalendar";
import { BracketGrid } from "../../components/bracket/BracketGrid";
import { BracketList } from "../../components/bracket/BracketList";
import { BracketSearch, useBracketSearch } from "../../components/bracket/BracketSearch";
import {
  SelectedMatchBar,
  SubscribeButton,
  useSubscriptions,
} from "../../components/bracket/MatchSubscription";
import {
  BracketViewTabs,
  CountdownToggle,
  useBracketView,
  useCountdownMode,
  useLinkedMatch,
} from "../../components/bracket/BracketViewTabs";
import { ErrorAlert } from "../../components/ErrorAlert";
import { withScore } from "../../lib/bracket";
import { useLiveRefresh } from "../../lib/useLiveRefresh";
import { useLiveBracket } from "../../live/LiveContext";

export function BracketPage() {
  const { user } = useAuth();
  const [data, setData] = useState<PublicBracket | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useBracketView("public");
  const [countdown, toggleCountdown] = useCountdownMode("public");
  // Selecting a match only highlights it and the lines into and out of it.
  const [selectedId, setSelectedId] = useState<string | null>(null);
  // The match a view opens on, centred or scrolled to (a new object each time): switching views passes the
  // selected one, so the match last clicked in one view is where the next one starts.
  const [focus, setFocus] = useState<{ id: string } | null>(null);
  const openView = (next: typeof view, id: string | null) => {
    if (id) setSelectedId(id);
    setFocus(id ? { id } : null);
    setView(next);
  };
  const pickView = (next: typeof view) => openView(next, selectedId);
  const showInCalendar = (id: string) => openView("calendar", id);
  // A team's match history links to a match on the grid.
  useLinkedMatch((id) => openView("grid", id));

  const reload = useCallback(async () => setData(await fetchPublicBracket()), []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  const teams = useMemo(() => new Map<string, Team>(data?.teams.map((t) => [t.id, t])), [data]);
  const matches = useMemo(() => data?.matches ?? [], [data]);
  // A failed refresh keeps what's shown; the next one tries again.
  useLiveRefresh(matches, () => reload().catch(() => {}));
  useLiveBracket(
    (e) =>
      setData(
        (d) =>
          d && { ...d, matches: withScore(d.matches, e.match_id, e.team1_score, e.team2_score) },
      ),
    () => reload().catch(() => {}),
  );
  const search = useBracketSearch(matches, teams);
  // Signed-in users can ask to be pinged on Discord when a match is about to start.
  const subs = useSubscriptions(user != null);
  const selected = matches.find((m) => m.id === selectedId);
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
      <ErrorAlert message={subs.error} onDismiss={subs.clearError} />
      {!data && !error && <p className="status">Yükleniyor…</p>}
      {data?.matches.length === 0 && <p className="status">Bracket henüz hazırlanmadı.</p>}

      {data && data.matches.length > 0 && (
        <>
          {view !== "list" && <SelectedMatchBar match={selected} teams={teams} subs={subs} />}
          {view === "grid" && (
            <BracketGrid
              {...shared}
              centerOn={focus}
              onSelect={setSelectedId}
              onShowInCalendar={showInCalendar}
              countdown={countdown}
            />
          )}
          {view === "list" && (
            <BracketList
              {...shared}
              focusOn={focus}
              onSelect={setSelectedId}
              onShowInCalendar={showInCalendar}
              countdown={countdown}
              renderActions={
                subs.ids ? (m) => <SubscribeButton match={m} subs={subs} /> : undefined
              }
            />
          )}
          {view === "calendar" && (
            <BracketCalendar {...shared} focusId={focus?.id} onSelect={setSelectedId} />
          )}
        </>
      )}
    </div>
  );
}
