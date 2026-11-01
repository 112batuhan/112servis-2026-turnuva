import type { ReactNode } from "react";
import { API_URL } from "../api/client";
import type { Lobby, Player, Team } from "../api/types";
import { isLobbyLive } from "../lib/bracket";
import { formatMatchTime } from "../lib/format";
import { MatchPageLink } from "./MatchPageLink";
import { RoomLinks } from "./RoomLinks";
import { StatusChip } from "./StatusChip";
import { Countdown } from "./Countdown";
import { DiscordMention } from "./DiscordMention";
import { TeamPopover } from "./TeamPopover";
import "./LobbyCard.css";

interface Props<P extends Player> {
  lobby: Lobby<P>;
  capacity: number;
  /** Marks the viewer's own lobby. */
  highlight?: boolean;
  /** Called when the lobby's start time arrives, so the page can reload it as started. */
  onStart?: () => void;
  /** Extra content under each player's name, e.g. their Discord for staff. */
  renderPlayerExtra?: (player: P) => ReactNode;
  /** Controls at the end of each team row. */
  renderTeamActions?: (team: Team<P>) => ReactNode;
  /** Replaces the read-only referee and link line, e.g. with staff controls. */
  children?: ReactNode;
  actions?: ReactNode;
}

export function LobbyCard<P extends Player>({
  lobby,
  capacity,
  highlight,
  onStart,
  renderPlayerExtra,
  renderTeamActions,
  children,
  actions,
}: Props<P>) {
  const full = lobby.teams.length >= capacity;

  return (
    <article className={highlight ? "lobby-card is-mine" : "lobby-card"}>
      <header className="lobby-card-header">
        <div className="lobby-card-title">
          <h3>Lobi {lobby.name}</h3>
          <LobbyStatus lobby={lobby} full={full} />
          <span className={full ? "lobby-card-count is-full" : "lobby-card-count"} title="Kayıtlı takımlar">
            {lobby.teams.length}/{capacity}
          </span>
        </div>
        <div className="lobby-card-when">
          <time className="lobby-card-time" dateTime={lobby.starts_at}>
            {formatMatchTime(lobby.starts_at)}
          </time>
          {lobby.open && (
            <Countdown to={lobby.starts_at} suffix="kaldı" className="lobby-card-countdown" onElapsed={onStart} />
          )}
        </div>
      </header>

      {children ?? (
        <div className="lobby-card-meta">
          <dl>
            <dt>Hakem</dt>
            <dd>
              {lobby.referee ? (
                <>
                  {lobby.referee.username}{" "}
                  <span className="lobby-card-discord">
                    <DiscordMention player={lobby.referee} fallback="" />
                  </span>
                </>
              ) : (
                <span className="muted">Henüz yok</span>
              )}
            </dd>
          </dl>
          <div className="lobby-card-links">
            <MatchPageLink matchId={lobby.id} live={isLobbyLive(lobby)} variant="text" />
            <RoomLinks rooms={lobby.rooms} />
          </div>
        </div>
      )}

      <ol className="lobby-card-teams">
        {lobby.teams.map((team) => (
          <li key={team.id} className="lobby-team">
            <div className="lobby-team-banner">
              {team.banner_url && <img src={`${API_URL}${team.banner_url}`} alt="" loading="lazy" />}
            </div>
            <div className="lobby-team-text">
              <div className="lobby-team-main">
                <TeamPopover team={team} className="lobby-team-name">
                  <span className="lobby-team-name-text">{team.name}</span>
                </TeamPopover>
                <div className="lobby-team-players">
                  {team.players.map((p) => (
                    <div key={p.id} className="lobby-player">
                      {p.avatar_url && <img className="lobby-player-avatar" src={p.avatar_url} alt="" loading="lazy" />}
                      <div className="lobby-player-text">
                        <a
                          className="lobby-player-name"
                          href={`https://osu.ppy.sh/users/${p.osu_id}`}
                          target="_blank"
                          rel="noopener noreferrer"
                        >
                          {p.username}
                        </a>
                        {renderPlayerExtra && <span className="lobby-player-extra">{renderPlayerExtra(p)}</span>}
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            </div>
            {renderTeamActions && <div className="lobby-team-actions">{renderTeamActions(team)}</div>}
          </li>
        ))}
        {/* The free places as one line, not a row each. */}
        {!full && (
          <li className="lobby-free">
            {lobby.teams.length === 0 ? `${capacity} yerin hepsi boş` : `${capacity - lobby.teams.length} yer daha boş`}
          </li>
        )}
      </ol>

      {actions && <div className="lobby-card-actions">{actions}</div>}
    </article>
  );
}

export function LobbyStatus({ lobby, full }: { lobby: Lobby<Player>; full: boolean }) {
  if (lobby.concluded_at) return <StatusChip state="done">Tamamlandı</StatusChip>;
  if (isLobbyLive(lobby)) return <StatusChip state="live">Canlı</StatusChip>;
  if (!lobby.open) return <StatusChip state="due">Başladı</StatusChip>;
  if (full) return <StatusChip state="full">Dolu</StatusChip>;
  return null;
}
