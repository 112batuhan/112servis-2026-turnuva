import { useEffect, useState } from "react";
import { errorMessage } from "../api/client";
import { fetchPublicTeams } from "../api/teams";
import type { Team } from "../api/types";
import { ErrorAlert } from "../components/ErrorAlert";
import { TeamCard } from "../components/TeamCard";

export function TeamsPage() {
  const [teams, setTeams] = useState<Team[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchPublicTeams()
      .then(setTeams)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Teams</h1>
        {teams && <span className="badge">{teams.length}</span>}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!teams && !error && <p className="status">Loading…</p>}
      {teams?.length === 0 && <p className="status">No teams yet.</p>}
      {teams && teams.length > 0 && (
        <div className="team-grid">
          {teams.map((team) => (
            <TeamCard key={team.id} team={team} />
          ))}
        </div>
      )}
    </div>
  );
}
