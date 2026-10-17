import { useCallback, useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import { disbandTeam, fetchTeams, refreshRanks, removeTeamBanner, renameTeam, setTeamsFrozen } from "../../api/teams";
import type { AdminTeamPlayer, Team } from "../../api/types";
import { DiscordMention } from "../../components/DiscordMention";
import { ErrorAlert } from "../../components/ErrorAlert";
import { RanksUpdated } from "../../components/RanksUpdated";
import { TeamCard } from "../../components/TeamCard";
import { TeamRow, TeamRows } from "../../components/TeamRow";
import { TeamViewTabs, useTeamView } from "../../components/TeamViewTabs";

export function TeamsAdminPage() {
  const [teams, setTeams] = useState<Team<AdminTeamPlayer>[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [ranksUpdatedAt, setRanksUpdatedAt] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);
  const [frozen, setFrozen] = useState(false);
  const [freezing, setFreezing] = useState(false);
  const [view, setView] = useTeamView("admin");
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
      ? "Takımların dondurulması kaldırılsın mı? Oyuncular yeniden takım kurabilir, değiştirebilir ve dağıtabilir; rank'lar da yeniden güncellenmeye başlar."
      : "Takımlar dondurulsun mu? Oyuncular takım kuramaz, takımın adını veya banner'ını değiştiremez ve takımı dağıtamaz; dondurmayı kaldırana kadar rank'lar da güncellenmez.";
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
    if (!window.confirm(`${team.name} takımının banner'ı kaldırılsın mı?`)) return;
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
    if (!window.confirm(`${team.name} takımı dağıtılsın mı?`)) return;
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

  // The same team, its controls and its players' Discords, for a card or a row.
  const teamProps = (team: Team<AdminTeamPlayer>, i: number) => ({
    team,
    position: i + 1,
    onRename: (name: string) => rename(team, name),
    renderPlayerExtra: (p: AdminTeamPlayer) => (
      <span className="muted">
        <DiscordMention player={p} fallback="Discord bağlı değil" />
        {p.discord_in_guild === false && (
          <>
            {" "}
            <span className="badge badge-warning" title="Turnuvanın Discord sunucusundan ayrıldı">
              Sunucuda değil
            </span>
          </>
        )}
      </span>
    ),
    actions: (
      <>
        {team.banner_url && (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={busyId === team.id}
            onClick={() => removeBanner(team)}
          >
            Banner'ı kaldır
          </button>
        )}
        <button
          type="button"
          className="btn btn-danger btn-sm"
          disabled={busyId === team.id}
          onClick={() => disband(team)}
        >
          Dağıt
        </button>
      </>
    ),
  });

  return (
    <section className="panel">
      <div className="page-header">
        <h1>Takımlar</h1>
        {teams && <span className="badge">{teams.length}</span>}
        {frozen && (
          <span className="badge badge-success" title="Oyuncular takımları değiştiremez">
            Donduruldu
          </span>
        )}
        {flagged > 0 && (
          <span className="badge badge-danger" title="Güncel rank'ları artık limite uymayan takımlar">
            {flagged} işaretli
          </span>
        )}
        {teams && (
          <div className="page-header-aside">
            <RanksUpdated at={ranksUpdatedAt} />
            <TeamViewTabs view={view} onChange={setView} />
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={refreshing || frozen}
              title={frozen ? "Takımlar donduruluyken rank'lar güncellenmez" : undefined}
              onClick={updateRanks}
            >
              {refreshing ? "Güncelleniyor…" : "Rank'ları şimdi güncelle"}
            </button>
            <button
              type="button"
              className={frozen ? "btn btn-secondary btn-sm" : "btn btn-primary btn-sm"}
              disabled={freezing}
              onClick={toggleFrozen}
            >
              {frozen ? "Dondurmayı kaldır" : "Takımları dondur"}
            </button>
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!teams && !error && <p className="status">Yükleniyor…</p>}
      {teams?.length === 0 && <p className="status">Henüz takım yok.</p>}
      {teams && teams.length > 0 && view === "cards" && (
        <div className="team-grid is-ruled">
          {teams.map((team, i) => (
            <TeamCard key={team.id} {...teamProps(team, i)} />
          ))}
        </div>
      )}
      {teams && teams.length > 0 && view === "rows" && (
        <TeamRows numbered>
          {teams.map((team, i) => (
            <TeamRow key={team.id} {...teamProps(team, i)} />
          ))}
        </TeamRows>
      )}
    </section>
  );
}
