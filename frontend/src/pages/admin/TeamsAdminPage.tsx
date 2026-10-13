import { useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import { disbandTeam, fetchTeams } from "../../api/teams";
import type { AdminTeamPlayer, Team } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import { TeamCard } from "../../components/TeamCard";

export function TeamsAdminPage() {
  const [teams, setTeams] = useState<Team<AdminTeamPlayer>[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);

  useEffect(() => {
    fetchTeams()
      .then(setTeams)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  async function disband(team: Team<AdminTeamPlayer>) {
    const names = team.players.map((p) => p.username).join(" & ");
    if (!window.confirm(`Disband ${names}?`)) return;
    setError(null);
    setBusyId(team.id);
    try {
      await disbandTeam(team.id);
      setTeams((list) => list?.filter((t) => t.id !== team.id) ?? null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  return (
    <section className="panel">
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
            <TeamCard
              key={team.id}
              team={team}
              renderPlayerExtra={(p) => (
                <span className="muted">{p.discord_username ? `@${p.discord_username}` : "No Discord linked"}</span>
              )}
              actions={
                <button
                  type="button"
                  className="btn btn-danger btn-sm"
                  disabled={busyId === team.id}
                  onClick={() => disband(team)}
                >
                  Disband
                </button>
              }
            />
          ))}
        </div>
      )}
    </section>
  );
}
