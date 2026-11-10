import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../api/client";
import { useAction } from "../lib/useAction";
import { fetchPublicQualifiers, joinLobby, leaveLobby } from "../api/matches";
import type { Lobby, PublicQualifiers } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { canPlay } from "../auth/roles";
import { Countdown } from "../components/Countdown";
import { ErrorAlert } from "../components/ErrorAlert";
import { LobbyCard } from "../components/LobbyCard";
import { formatMatchTime } from "../lib/format";
import "./QualifiersPage.css";

export function QualifiersPage() {
  const { user } = useAuth();
  const [data, setData] = useState<PublicQualifiers | null>(null);
  const { busy, error, setError, run } = useAction(() => reload());

  const reload = useCallback(async () => setData(await fetchPublicQualifiers()), []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  const lobbies = data?.lobbies;
  const myTeamId = data?.my_team_id ?? null;
  const myLobby = (myTeamId ? lobbies?.find((l) => l.teams.some((t) => t.id === myTeamId)) : undefined) ?? null;
  // Once their lobby has started a team stays where it is.
  const canMove = myTeamId !== null && (!myLobby || myLobby.open);

  function lobbyActions(lobby: Lobby) {
    if (!myTeamId) return undefined;
    if (lobby === myLobby) {
      return lobby.open ? (
        <button type="button" className="btn btn-secondary btn-sm" disabled={busy} onClick={() => run(leaveLobby)}>
          Lobiden ayrıl
        </button>
      ) : undefined;
    }
    if (!canMove || !lobby.open) return undefined;
    const full = lobby.teams.length >= data!.capacity;
    return (
      <button
        type="button"
        className="btn btn-primary btn-sm"
        disabled={busy || full}
        title={full ? "Bu lobi dolu" : undefined}
        onClick={() => run(() => joinLobby(lobby.id))}
      >
        {myLobby ? "Bu lobiye geç" : "Bu lobiye katıl"}
      </button>
    );
  }

  return (
    <div className="page qualifiers-page">
      <div className="page-header">
        <h1>Elemeler</h1>
        {lobbies && <span className="badge">{lobbies.length} lobi</span>}
        {data?.pool_stage_id && (
          <div className="page-header-aside">
            <Link to={`/mappool?stage=${data.pool_stage_id}`} className="btn btn-secondary btn-sm">
              Mappool
            </Link>
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!data && !error && <p className="status">Yükleniyor…</p>}

      {data && (
        <section className={myLobby ? "qualifiers-summary is-mine" : "qualifiers-summary"}>
          {myLobby ? (
            <>
              <p>
                Takımın <strong>Lobi {myLobby.name}</strong> için kayıtlı: {formatMatchTime(myLobby.starts_at)}.{" "}
                {myLobby.open
                  ? "Lobi başlayana kadar sen veya takım arkadaşın başka bir lobiye geçebilirsiniz."
                  : "Lobi başladığı için artık değiştirilemez."}
              </p>
              {myLobby.open && (
                <div className="qualifiers-countdown">
                  <span className="qualifiers-countdown-label">Başlamasına</span>
                  <Countdown to={myLobby.starts_at} className="qualifiers-countdown-time" />
                </div>
              )}
            </>
          ) : myTeamId ? (
            <p>
              Takımın henüz bir lobiye kayıtlı değil. Aşağıdan size uyan bir lobi seçin; her lobide en fazla{" "}
              {data.capacity} takım olabilir.
            </p>
          ) : canPlay(user) ? (
            <p>
              Lobi seçmek için önce bir <Link to="/team">takım kur</Link>.
            </p>
          ) : (
            <p>Her takım, en fazla {data.capacity} takımın oynadığı eleme lobilerinden birini seçer.</p>
          )}
        </section>
      )}

      {lobbies?.length === 0 && <p className="status">Henüz eleme lobisi açılmadı.</p>}
      {lobbies && lobbies.length > 0 && (
        <div className="lobby-grid">
          {lobbies.map((lobby) => (
            <LobbyCard
              key={lobby.id}
              lobby={lobby}
              capacity={data!.capacity}
              highlight={lobby === myLobby}
              onStart={() => reload().catch((e) => setError(errorMessage(e)))}
              actions={lobbyActions(lobby)}
            />
          ))}
        </div>
      )}
    </div>
  );
}
