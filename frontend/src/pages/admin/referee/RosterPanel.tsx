import type { AdminTeamPlayer, Team } from "../../../api/types";
import { DiscordMention } from "../../../components/DiscordMention";
import { IconButton } from "../../../components/IconButton";
import { sameName, type SlotTeam } from "../../../lib/bancho";
import { formatRank } from "../../../lib/format";
import { teamTotals, type RefereeRoomState } from "./useRefereeRoom";
import "./RosterPanel.css";

/** The match's teams and players with their Discords and where they are in the room. */
export function RosterPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, roster } = referee;
  if (!room) return null;
  const colorOf = (team: Team<AdminTeamPlayer>) => roster.find((r) => r.team.id === team.id)?.color ?? null;

  return (
    <section className="roster-panel" aria-label="Takımlar">
      <h2 className="roster-title">Takımlar</h2>
      {room.teams.length === 0 && <p className="muted">Bu maçta henüz takım yok.</p>}
      {room.teams.map((team) => (
        <TeamBlock key={team.id} team={team} color={colorOf(team)} referee={referee} />
      ))}
      <Results referee={referee} />
    </section>
  );
}

function TeamBlock({
  team,
  color,
  referee,
}: {
  team: Team<AdminTeamPlayer>;
  color: SlotTeam | null;
  referee: RefereeRoomState;
}) {
  const { lobby, actions, inChannel } = referee;
  return (
    <div className={`roster-team ${color ? `is-team-${color}` : ""}`}>
      <h3 className="roster-team-name">
        {team.name}
        {color && <span className="roster-team-color">{color === "red" ? "Kırmızı" : "Mavi"}</span>}
      </h3>
      <ul className="roster-players">
        {team.players.map((p) => {
          const slot = lobby.slots.find((s) => s.userId === p.osu_id || sameName(s.username, p.username));
          return (
            <li key={p.id} className="roster-player">
              <img className="roster-avatar" src={p.avatar_url} alt="" loading="lazy" />
              <span className="roster-player-text">
                <a href={`https://osu.ppy.sh/users/${p.osu_id}`} target="_blank" rel="noopener noreferrer">
                  {p.username}
                </a>
                <span className="roster-player-meta">
                  {formatRank(p.global_rank)} · <DiscordMention player={p} fallback="Discord yok" />
                </span>
              </span>
              {slot ? (
                <span className="badge badge-success" title="Odadaki slotu">
                  Slot {slot.slot}
                </span>
              ) : (
                <span className="badge">Odada değil</span>
              )}
              <IconButton
                icon="userPlus"
                label={`${p.username} oyuncusunu odaya davet et`}
                disabled={!inChannel}
                onClick={() => actions.invite(p.osu_id)}
              />
            </li>
          );
        })}
      </ul>
    </div>
  );
}

/** Each finished map's team totals; for a bracket match, the map wins and saving them as the score. */
function Results({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, roster, ignored, actions, canRun, busy } = referee;
  if (!room || lobby.results.length === 0) return null;
  const name = (teamId: string) => room.teams.find((t) => t.id === teamId)?.name ?? "?";
  const bracket = room.kind === "bracket" && room.team1_id !== null && room.team2_id !== null;

  const wins = new Map<string, number>();
  lobby.results.forEach((result, i) => {
    const [best, second] = teamTotals(result, roster);
    if (ignored.has(i) || !best || (second && second.score === best.score)) return;
    wins.set(best.teamId, (wins.get(best.teamId) ?? 0) + 1);
  });
  const team1Wins = room.team1_id ? (wins.get(room.team1_id) ?? 0) : 0;
  const team2Wins = room.team2_id ? (wins.get(room.team2_id) ?? 0) : 0;
  const saved = room.team1_score === team1Wins && room.team2_score === team2Wins;

  return (
    <div className="roster-results">
      <h3 className="roster-title">Sonuçlar</h3>
      <ol className="roster-result-list">
        {lobby.results.map((result, i) => {
          const totals = teamTotals(result, roster);
          return (
            <li key={i} className={ignored.has(i) ? "is-ignored" : undefined}>
              <span className="roster-result-map">{result.beatmapName ?? `Map ${i + 1}`}</span>
              <span className="roster-result-totals">
                {totals.length === 0 && <span className="muted">Skor yok</span>}
                {totals.map((t, j) => (
                  <span key={t.teamId} className={j === 0 && totals.length > 1 ? "is-winner" : undefined}>
                    {name(t.teamId)} {t.score.toLocaleString("tr-TR")}
                  </span>
                ))}
              </span>
              <label className="roster-result-ignore">
                <input type="checkbox" checked={ignored.has(i)} onChange={() => actions.toggleIgnored(i)} /> Sayma
              </label>
            </li>
          );
        })}
      </ol>
      {bracket && (
        <div className="roster-score">
          <span>
            {name(room.team1_id!)} <strong>{team1Wins}</strong> – <strong>{team2Wins}</strong> {name(room.team2_id!)}
          </span>
          {canRun && room.concluded_at === null && (
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy || saved}
              onClick={() => actions.saveScore(team1Wins, team2Wins)}
            >
              {saved ? "Skor kayıtlı" : "Skoru kaydet"}
            </button>
          )}
        </div>
      )}
    </div>
  );
}
