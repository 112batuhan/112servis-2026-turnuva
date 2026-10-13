import { useCallback, useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import { disbandTeam, fetchTeams, refreshRanks, removeTeamBanner, renameTeam, setTeamsFrozen } from "../../api/teams";
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
  const [frozen, setFrozen] = useState(false);
  const [freezing, setFreezing] = useState(false);
  const flagged = teams?.filter((t) => t.rank_warning).length ?? 0;

  const load = useCallback(async () => {
    const list = await fetchTeams();
    setTeams(list.teams);
    setRanksUpdatedAt(list.ranks_updated_at);
    setFrozen(list.frozen);
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

  async function toggleFrozen() {
    const message = frozen
      ? "Unfreeze the teams? Players can form, change and disband teams again."
      : "Freeze the teams? Players won't be able to form, rename, re-banner or disband teams until you unfreeze them.";
    if (!window.confirm(message)) return;
    setError(null);
    setFreezing(true);
    try {
      await setTeamsFrozen(!frozen);
      setFrozen(!frozen);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setFreezing(false);
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
        {frozen && (
          <span className="badge badge-success" title="Players can't change teams">
            Frozen
          </span>
        )}
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
            <button
              type="button"
              className={frozen ? "btn btn-secondary btn-sm" : "btn btn-primary btn-sm"}
              disabled={freezing}
              onClick={toggleFrozen}
            >
              {frozen ? "Unfreeze teams" : "Freeze teams"}
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
