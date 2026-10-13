import { useCallback, useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import { disbandTeam, fetchTeams, refreshRanks, removeTeamBanner, renameTeam } from "../../api/teams";
import type { AdminTeamPlayer, Team } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import { RanksUpdated } from "../../components/RanksUpdated";
import { TeamCard } from "../../components/TeamCard";

export function TeamsAdminPage() {
  const [teams, setTeams] = useState<Team<AdminTeamPlayer>[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [ranksUpdatedAt, setRanksUpdatedAt] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const flagged = teams?.filter((t) => t.rank_warning).length ?? 0;

  const load = useCallback(async () => {
    const list = await fetchTeams();
    setTeams(list.teams);
    setRanksUpdatedAt(list.ranks_updated_at);
  }, []);

  useEffect(() => {
    load().catch((e) => setError(errorMessage(e)));
  }, [load]);

  async function updateRanks() {
    setError(null);
    setRefreshing(true);
    try {
      await refreshRanks();
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setRefreshing(false);
    }
  }

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
        {flagged > 0 && (
          <span className="badge badge-danger" title="Teams whose current ranks no longer fit the limit">
            {flagged} flagged
          </span>
        )}
        {teams && (
          <div className="page-header-aside">
            <RanksUpdated at={ranksUpdatedAt} />
            <button type="button" className="btn btn-secondary btn-sm" disabled={refreshing} onClick={updateRanks}>
              {refreshing ? "Updating…" : "Update ranks now"}
            </button>
          </div>
        )}
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
