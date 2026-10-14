import { useCallback, useEffect, useMemo, useState } from "react";
import { errorMessage } from "../api/client";
import { fetchPublicBracket } from "../api/bracket";
import type { PublicBracket, Team } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { BracketCalendar } from "../components/bracket/BracketCalendar";
import { BracketGrid } from "../components/bracket/BracketGrid";
import { BracketList } from "../components/bracket/BracketList";
import { BracketViewTabs, useBracketView } from "../components/bracket/BracketViewTabs";
import { ErrorAlert } from "../components/ErrorAlert";

export function BracketPage() {
  const { user } = useAuth();
  const [data, setData] = useState<PublicBracket | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useBracketView("public");
  // Selecting a match only highlights it and the lines into and out of it.
  const [selectedId, setSelectedId] = useState<string | null>(null);

  const reload = useCallback(async () => setData(await fetchPublicBracket()), []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  const teams = useMemo(() => new Map<string, Team>(data?.teams.map((t) => [t.id, t])), [data]);
  const shared = {
    matches: data?.matches ?? [],
    teams,
    myTeamId: data?.my_team_id,
    viewerId: user?.id,
    selectedId,
  };

  return (
    <div className="page">
      <div className="page-header">
        <h1>Bracket</h1>
        {data && <span className="badge">{data.matches.length} maç</span>}
        {data && data.matches.length > 0 && (
          <div className="page-header-aside">
            <BracketViewTabs view={view} onChange={setView} />
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!data && !error && <p className="status">Yükleniyor…</p>}
      {data?.matches.length === 0 && <p className="status">Bracket henüz hazırlanmadı.</p>}

      {data && data.matches.length > 0 && (
        <>
          {view === "grid" && <BracketGrid {...shared} onSelect={setSelectedId} />}
          {view === "list" && <BracketList {...shared} />}
          {view === "calendar" && <BracketCalendar {...shared} />}
        </>
      )}
    </div>
  );
}
