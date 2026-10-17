import type { ReactNode } from "react";
import { Icon } from "../../../components/Icon";
import { IconButton } from "../../../components/IconButton";
import { stageLabel, type Race } from "../../../lib/bestOf";
import { formatLength } from "../../../lib/format";
import { BAN_LIMIT, entryOf, matchTeams, poolEntries, type MatchData, type MatchTeam, type PoolEntry } from "./pool";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./PoolPanel.css";

/**
 * The match's map pool. Clicking a map sets it and its mods in the room. In a bracket
 * match the referee also marks bans (until there are BAN_LIMIT of them) and then
 * which team picked which map; a pick sets the map too. Banned maps can't be picked.
 */
export function PoolPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, currentEntry, finished, inChannel, canRun, busy, actions, race } = referee;
  if (!room) return null;
  const entries = poolEntries(room);
  const picks = new Set(room.picks.map((p) => p.map_id));
  // By entry, so a beatmap that's in the pool twice is told apart by its mods.
  const played = new Set(finished.map((f) => entryOf(entries, f.result, picks)?.map.id));
  const teams = matchTeams(room);
  const canMark = teams.length === 2 && canRun && room.concluded_at === null;
  const banning = room.bans.length < BAN_LIMIT;

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
          disabled={busy || played.has(map.id)}
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
    <PoolView
      room={room}
      currentEntry={currentEntry}
      played={played}
      phase={canMark ? draftPhase(banning, race) : null}
      pick={{
        disabled: (_entry, banned) => !inChannel || banned !== undefined,
        title: (entry, banned) =>
          banned
            ? `${entry.label}: ${banned.name} yasakladı`
            : inChannel
              ? `${entry.label} seç: !mp map ${entry.map.beatmap_id}, ${entry.freemod ? "Freemod" : entry.map.mods || "NM"}`
              : "Önce odanın kanalına katıl",
        onPick: (entry) => actions.pickMap(entry.map, entry.freemod),
      }}
      renderActions={canMark ? markActions : undefined}
    />
  );
}

/** Where the draft stands: bans, picks, then the tiebreaker; nothing once a team has won. */
export function draftPhase(banning: boolean, race: Race | null): string | null {
  if (race?.kind === "won") return null;
  if (race?.kind === "tiebreaker") return "Tiebreaker";
  return banning ? "Ban sırası" : "Pick sırası";
}

interface PoolViewProps {
  room: MatchData;
  /** The map the room has on. */
  currentEntry: PoolEntry | undefined;
  /** The pool maps (PublicPoolMap.id) played so far. */
  played: Set<string | undefined>;
  /** Where the draft stands, beside the title. */
  phase?: string | null;
  /**
   * Makes each map a button that sets it in the room (the referee page). Without it each map
   * links to its beatmap on osu!.
   */
  pick?: {
    disabled: (entry: PoolEntry, banned: MatchTeam | undefined) => boolean;
    title: (entry: PoolEntry, banned: MatchTeam | undefined) => string;
    onPick: (entry: PoolEntry) => void;
  };
  /** The referee's ban and pick buttons at the end of each map. */
  renderActions?: (entry: PoolEntry, banned: MatchTeam | undefined, picked: MatchTeam | undefined) => ReactNode;
}

/**
 * The match's pool with the draft so far: the bans and picks as team-coloured chips, then every
 * map with what happened to it (banned, picked, on in the room, played). Shared by the referee
 * page and the public match page.
 */
export function PoolView({ room, currentEntry, played, phase, pick, renderActions }: PoolViewProps) {
  const entries = poolEntries(room);
  const teams = matchTeams(room);
  const teamOf = (id: string) => teams.find((t) => t.id === id);
  const bannedBy = new Map(room.bans.map((b) => [b.map_id, teamOf(b.team_id)]));
  const pickedBy = new Map(room.picks.map((p) => [p.map_id, teamOf(p.team_id)]));
  const labelOf = new Map(entries.map((e) => [e.map.id, e.label]));

  return (
    <section className="pool-panel" aria-label="Mappool">
      <h2 className="pool-panel-title">
        Mappool
        {room.stage && (
          <span className="pool-panel-stage">{room.kind === "bracket" ? stageLabel(room.stage) : room.stage.name}</span>
        )}
        {phase && <span className="pool-panel-phase">{phase}</span>}
      </h2>
      {!room.stage && <p className="pool-panel-note">Bu maça stage seçilmemiş.</p>}
      {room.stage && entries.length === 0 && (
        <p className="pool-panel-note">
          {pick || room.stage.published ? "Bu stage'in havuzunda map yok." : "Bu stage'in havuzu henüz yayımlanmadı."}
        </p>
      )}

      {teams.length === 2 && entries.length > 0 && (
        <dl className="pool-draft" aria-label="Yasaklar ve seçimler">
          {[
            { kind: "Yasak", marks: room.bans, limit: `${room.bans.length}/${BAN_LIMIT}` },
            { kind: "Seçim", marks: room.picks, limit: String(room.picks.length) },
          ].map(({ kind, marks, limit }) => (
            <div key={kind} className="pool-draft-row">
              <dt>
                {kind} <span className="pool-draft-count">{limit}</span>
              </dt>
              <dd>
                {marks.length === 0 && <span className="pool-draft-none">Henüz yok</span>}
                {marks.map((m) => {
                  const team = teamOf(m.team_id);
                  return (
                    <span
                      key={m.map_id}
                      className={`pool-draft-chip is-team-${team?.color ?? "none"}${kind === "Yasak" ? " is-ban" : ""}`}
                      title={`${team?.name ?? "?"} ${kind === "Yasak" ? "yasakladı" : "seçti"}`}
                    >
                      {labelOf.get(m.map_id) ?? "?"}
                    </span>
                  );
                })}
              </dd>
            </div>
          ))}
        </dl>
      )}

      <ul className="pool-panel-list">
        {entries.map((entry) => {
          const { label, color, freemod, map } = entry;
          const current = currentEntry?.map.id === map.id;
          const banned = bannedBy.get(map.id);
          const picked = pickedBy.get(map.id);
          const body = (
            <>
              <span className="pool-pick-label" style={{ borderColor: color, color }}>
                {label}
              </span>
              <span className="pool-pick-text">
                <span className="pool-pick-title">
                  {map.artist} - {map.title}
                </span>
                <span className="pool-pick-meta">
                  <Icon name="star" size={11} className="pool-pick-star" />
                  {map.star_rating.toFixed(2)} · {formatLength(map.total_length)} · {Math.round(map.bpm)} BPM
                  {freemod && " · Freemod"} · [{map.version}]
                </span>
              </span>
              {banned && (
                <span className={`pool-mark is-team-${banned.color}`} title={`${banned.name} yasakladı`}>
                  yasak
                </span>
              )}
              {picked && (
                <span className={`pool-mark is-team-${picked.color}`} title={`${picked.name} seçti`}>
                  seçim
                </span>
              )}
              {!banned && !picked && current && <span className="pool-mark is-current">Odada</span>}
              {!banned && !picked && !current && played.has(map.id) && <span className="pool-mark">Oynandı</span>}
            </>
          );
          return (
            <li
              key={map.id}
              className={[
                "pool-entry",
                current && "is-current",
                played.has(map.id) && "is-played",
                banned && `is-banned is-marked-${banned.color}`,
                picked && `is-marked-${picked.color}`,
              ]
                .filter(Boolean)
                .join(" ")}
            >
              {pick ? (
                <button
                  type="button"
                  className="pool-pick"
                  disabled={pick.disabled(entry, banned)}
                  title={pick.title(entry, banned)}
                  onClick={() => pick.onPick(entry)}
                >
                  {body}
                </button>
              ) : (
                <a
                  className="pool-pick"
                  href={`https://osu.ppy.sh/b/${map.beatmap_id}`}
                  target="_blank"
                  rel="noopener noreferrer"
                  title={`${label}: osu!'da aç`}
                >
                  {body}
                </a>
              )}
              {renderActions && <span className="pool-mark-actions">{renderActions(entry, banned, picked)}</span>}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
