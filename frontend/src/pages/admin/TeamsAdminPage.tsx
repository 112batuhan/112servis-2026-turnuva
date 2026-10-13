import { useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import { disbandTeam, fetchTeams, removeTeamBanner, renameTeam } from "../../api/teams";
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

  async function rename(team: Team<AdminTeamPlayer>, name: string) {
    setError(null);
    try {
      await renameTeam(team.id, name);
      setTeams((list) => list?.map((t) => (t.id === team.id ? { ...t, name } : t)) ?? null);
    } catch (e) {
      setError(errorMessage(e));
    }
  }

  async function removeBanner(team: Team<AdminTeamPlayer>) {
    if (!window.confirm(`Remove the banner of ${team.name}?`)) return;
    setError(null);
    setBusyId(team.id);
    try {
      await removeTeamBanner(team.id);
      setTeams((list) => list?.map((t) => (t.id === team.id ? { ...t, banner_url: null } : t)) ?? null);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusyId(null);
    }
  }

  async function disband(team: Team<AdminTeamPlayer>) {
    if (!window.confirm(`Disband ${team.name}?`)) return;
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
              onRename={(name) => rename(team, name)}
              renderPlayerExtra={(p) => (
                <span className="muted">{p.discord_username ? `@${p.discord_username}` : "No Discord linked"}</span>
              )}
              actions={
                <>
                  {team.banner_url && (
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm"
                      disabled={busyId === team.id}
                      onClick={() => removeBanner(team)}
                    >
                      Remove banner
                    </button>
                  )}
                  <button
                    type="button"
                    className="btn btn-danger btn-sm"
                    disabled={busyId === team.id}
                    onClick={() => disband(team)}
                  >
                    Disband
                  </button>
                </>
              }
            />
          ))}
        </div>
      )}
    </section>
  );
}
