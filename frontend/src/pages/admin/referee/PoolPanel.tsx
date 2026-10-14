import { IconButton } from "../../../components/IconButton";
import { formatLength } from "../../../lib/format";
import { BAN_LIMIT, matchTeams, poolEntries, type MatchTeam, type PoolEntry } from "./pool";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./PoolPanel.css";

/**
 * The match's map pool. Clicking a map sets it and its mods in the room. In a bracket
 * match the referee also marks bans (until there are BAN_LIMIT of them) and then
 * which team picked which map; a pick sets the map too. Banned maps can't be picked.
 */
export function PoolPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, inChannel, canRun, busy, actions } = referee;
  if (!room) return null;
  const entries = poolEntries(room);
  const played = new Set(lobby.results.map((r) => r.beatmapId));
  const teams = matchTeams(room);
  const teamOf = (id: string) => teams.find((t) => t.id === id);
  const bannedBy = new Map(room.bans.map((b) => [b.map_id, teamOf(b.team_id)]));
  const pickedBy = new Map(room.picks.map((p) => [p.map_id, teamOf(p.team_id)]));
  const canMark = teams.length === 2 && canRun && room.concluded_at === null;
  const banning = room.bans.length < BAN_LIMIT;
  const labelOf = new Map(entries.map((e) => [e.map.id, e.label]));

  // A ban or pick to take back, or the buttons for whichever phase the match is in.
  function markActions(entry: PoolEntry, banned: MatchTeam | undefined, picked: MatchTeam | undefined) {
    const { label, map, freemod, tiebreaker } = entry;
    if (banned) {
      return <IconButton icon="undo" label={`${label} yasağını kaldır`} disabled={busy} onClick={() => actions.unban(map.id)} />;
    }
    if (picked) {
      return <IconButton icon="undo" label={`${label} seçimini kaldır`} disabled={busy} onClick={() => actions.unpick(map.id)} />;
    }
    if (banning) {
      return teams.map((team) => (
        <IconButton
          key={team.id}
          icon="ban"
          className={`pool-mark-button is-team-${team.color}`}
          label={`${label}: ${team.name} yasakladı`}
          disabled={busy || played.has(map.beatmap_id)}
          onClick={() => actions.ban(map.id, team.id)}
        />
      ));
    }
    // Nobody picks the tiebreaker.
    if (tiebreaker) return null;
    return teams.map((team) => (
      <IconButton
        key={team.id}
        icon="target"
        className={`pool-mark-button is-team-${team.color}`}
        label={`${label}: ${team.name} seçti`}
        disabled={busy}
        onClick={() => actions.pick(map, team.id, freemod)}
      />
    ));
  }

  return (
    <section className="pool-panel" aria-label="Mappool">
      <h2 className="pool-panel-title">
        Mappool
        {room.stage && <span className="muted"> · {room.stage.name}</span>}
      </h2>
      {!room.stage && <p className="muted">Bu maça stage seçilmemiş.</p>}
      {room.stage && entries.length === 0 && <p className="muted">Bu stage'in havuzunda map yok.</p>}

      {teams.length === 2 && entries.length > 0 && (
        <dl className="pool-bans" aria-label="Yasaklar ve seçimler">
          {teams.map((team) => {
            const labels = (marks: { map_id: string; team_id: string }[]) =>
              marks.filter((m) => m.team_id === team.id).map((m) => labelOf.get(m.map_id) ?? "?");
            const bans = labels(room.bans);
            const picks = labels(room.picks);
            return (
              <div key={team.id} className={`pool-bans-team is-team-${team.color}`}>
                <dt>{team.name}</dt>
                <dd>
                  <span className="pool-bans-kind">Yasak</span> {bans.length > 0 ? bans.join(", ") : "—"}
                  <span className="pool-bans-kind">Seçim</span> {picks.length > 0 ? picks.join(", ") : "—"}
                </dd>
              </div>
            );
          })}
        </dl>
      )}

      <ul className="pool-panel-list">
        {entries.map((entry) => {
          const { label, color, freemod, map } = entry;
          const current = lobby.beatmapId === map.beatmap_id;
          const banned = bannedBy.get(map.id);
          const picked = pickedBy.get(map.id);
          const mods = freemod ? "Freemod" : map.mods || "NM";
          const pickTitle = banned
            ? `${label}: ${banned.name} yasakladı`
            : inChannel
              ? `${label} seç: !mp map ${map.beatmap_id}, ${mods}`
              : "Önce odanın kanalına katıl";
          return (
            <li key={map.id} className="pool-entry">
              <button
                type="button"
                className={[
                  "pool-pick",
                  current && "is-current",
                  played.has(map.beatmap_id) && "is-played",
                  banned && `is-banned is-marked-${banned.color}`,
                  picked && `is-marked-${picked.color}`,
                ]
                  .filter(Boolean)
                  .join(" ")}
                disabled={!inChannel || banned !== undefined}
                title={pickTitle}
                onClick={() => actions.pickMap(map, freemod)}
              >
                <span className="pool-pick-label" style={{ borderColor: color, color }}>
                  {label}
                </span>
                <span className="pool-pick-text">
                  <span className="pool-pick-title">
                    {map.artist} - {map.title}
                  </span>
                  <span className="pool-pick-meta">
                    [{map.version}] · {map.star_rating.toFixed(2)}★ · {Math.round(map.bpm)} BPM ·{" "}
                    {formatLength(map.total_length)} · {mods}
                  </span>
                </span>
                {banned && (
                  <span className={`badge pool-mark-badge is-team-${banned.color}`} title={`${banned.name} yasakladı`}>
                    Yasak
                  </span>
                )}
                {picked && (
                  <span className={`badge pool-mark-badge is-team-${picked.color}`} title={`${picked.name} seçti`}>
                    Seçim
                  </span>
                )}
                {!banned && !picked && current && <span className="badge badge-info">Seçili</span>}
                {!banned && !picked && !current && played.has(map.beatmap_id) && <span className="badge">Oynandı</span>}
              </button>
              {canMark && <span className="pool-mark-actions">{markActions(entry, banned, picked)}</span>}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
