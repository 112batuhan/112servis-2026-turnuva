import { useEffect, useState } from "react";
import { useLocation } from "react-router-dom";
import { errorMessage } from "../../api/client";
import { fetchPublicTeams } from "../../api/teams";
import type { TeamList } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import { RanksUpdated } from "../../components/RanksUpdated";
import { TeamCard } from "../../components/TeamCard";
import { TeamRow, TeamRows } from "../../components/TeamRow";
import { TeamViewTabs, useTeamView } from "../../components/TeamViewTabs";
import { teamAnchorId } from "../../lib/teamLink";

export function TeamsPage() {
  const [list, setList] = useState<TeamList | null>(null);
  const teams = list?.teams;
  const [error, setError] = useState<string | null>(null);
  const [view, setView] = useTeamView("public");
  // The qualifier column shows once any team has a place in it.
  const seeded = teams?.some((t) => t.qualifier) ?? false;
  // Set when another page linked to a team (/teams#team-<id>).
  const target = useLocation().hash.slice(1);

  useEffect(() => {
    fetchPublicTeams()
      .then(setList)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  useEffect(() => {
    if (teams && target)
      document.getElementById(target)?.scrollIntoView({ behavior: "smooth", block: "center" });
  }, [teams, target]);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Takımlar</h1>
        {teams && <span className="badge">{teams.length}</span>}
        {list?.frozen && (
          <span className="badge badge-success" title="Takımlar kesinleşti">
            Kesinleşti
          </span>
        )}
        {list && (
          <div className="page-header-aside">
            <RanksUpdated at={list.ranks_updated_at} />
            <TeamViewTabs view={view} onChange={setView} />
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!teams && !error && <p className="status">Yükleniyor…</p>}
      {teams?.length === 0 && <p className="status">Henüz takım yok.</p>}
      {teams && teams.length > 0 && view === "cards" && (
        <div className="team-grid is-ruled">
          {teams.map((team) => (
            <TeamCard
              key={team.id}
              team={team}
              qualifier={seeded ? team.qualifier : undefined}
              highlight={teamAnchorId(team.id) === target}
            />
          ))}
        </div>
      )}
      {teams && teams.length > 0 && view === "rows" && (
        <TeamRows qualifier={seeded}>
          {teams.map((team) => (
            <TeamRow
              key={team.id}
              team={team}
              qualifier={seeded ? team.qualifier : undefined}
              highlight={teamAnchorId(team.id) === target}
            />
          ))}
        </TeamRows>
      )}
    </div>
  );
}
