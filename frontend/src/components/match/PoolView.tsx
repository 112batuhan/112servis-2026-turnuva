import type { ReactNode } from "react";
import { Icon } from "../Icon";
import { stageLabel, type Race } from "../../lib/bestOf";
import { stepLabel, type Draft, type DraftStep } from "../../lib/draftOrder";
import { formatLength } from "../../lib/format";
import {
  matchTeams,
  poolEntries,
  type DraftTurn,
  type MatchData,
  type MatchTeam,
  type PoolEntry,
} from "../../lib/matchPool";
import "./PoolView.css";

/** Where the draft stands: a ban or a pick to make, then the tiebreaker; nothing once a team has won. */
export function draftPhase(turn: DraftTurn | null, race: Race | null): string | null {
  if (race?.kind === "won") return null;
  if (race?.kind === "tiebreaker") return "Tiebreaker";
  return turn ? `${stepLabel(turn.kind)} sırası` : null;
}

/** The next ban or pick in a phone tab's status: the step, and the team in its colour once known. */
export function TurnStatus({ turn }: { turn: DraftTurn }) {
  return (
    <span>
      {stepLabel(turn.kind)}
      {turn.team ? (
        <>
          {" · "}
          <span className={`referee-tab-team is-team-${turn.team.color}`}>{turn.team.name}</span>
        </>
      ) : (
        " sırası"
      )}
    </span>
  );
}

interface PoolViewProps {
  room: MatchData;
  /** The map the room has on. */
  currentEntry: PoolEntry | undefined;
  /** The pool maps (PublicPoolMap.id) played so far. */
  played: Set<string | undefined>;
  /** The bans and picks against the stage's order (a bracket match with both teams). */
  draft?: Draft | null;
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
  renderActions?: (
    entry: PoolEntry,
    banned: MatchTeam | undefined,
    picked: MatchTeam | undefined,
  ) => ReactNode;
}

/**
 * The match's pool with the draft so far: the bans and picks as team-coloured chips, then every
 * map with what happened to it (banned, picked, on in the room, played). Shared by the referee
 * page and the public match page.
 */
export function PoolView({
  room,
  currentEntry,
  played,
  draft,
  phase,
  pick,
  renderActions,
}: PoolViewProps) {
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
          <span className="pool-panel-stage">
            {room.kind === "bracket" ? stageLabel(room.stage) : room.stage.name}
          </span>
        )}
        {phase && <span className="pool-panel-phase">{phase}</span>}
      </h2>
      {!room.stage && <p className="pool-panel-note">Bu maça stage seçilmemiş.</p>}
      {room.stage && entries.length === 0 && (
        <p className="pool-panel-note">
          {pick || room.stage.published
            ? "Bu stage'in havuzunda map yok."
            : "Bu stage'in havuzu henüz yayımlanmadı."}
        </p>
      )}

      {draft && draft.steps.length > 0 && entries.length > 0 && (
        <DraftStrip draft={draft} teamOf={teamOf} labelOf={labelOf} />
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
                  {map.star_rating.toFixed(2)} · {formatLength(map.total_length)} ·{" "}
                  {Math.round(map.bpm)} BPM
                  {freemod && " · Freemod"} · [{map.version}]
                </span>
              </span>
              {banned && (
                <span
                  className={`pool-mark is-team-${banned.color}`}
                  title={`${banned.name} yasakladı`}
                >
                  <Icon name="ban" size={14} />
                  <span className="sr-only">{`${banned.name} yasakladı`}</span>
                </span>
              )}
              {picked && (
                <span
                  className={`pool-mark is-team-${picked.color}`}
                  title={`${picked.name} seçti`}
                >
                  <Icon name="target" size={14} />
                  <span className="sr-only">{`${picked.name} seçti`}</span>
                </span>
              )}
              {!banned && !picked && current && <span className="pool-mark is-current">Odada</span>}
              {!banned && !picked && !current && played.has(map.id) && (
                <span className="pool-mark">Oynandı</span>
              )}
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
              {renderActions && (
                <span className="pool-mark-actions">{renderActions(entry, banned, picked)}</span>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}

/**
 * The draft as the stage's order lays it out, one chip per step: the bans and picks made (their
 * pool label in the colour of the team that made it, bans struck through), the one whose turn it
 * is ringed, and the ones to come, coloured once the first mark has said who 1 is. A closing
 * "sırayla" says picks take turns from there.
 */
function DraftStrip({
  draft,
  teamOf,
  labelOf,
}: {
  draft: Draft;
  teamOf: (id: string) => MatchTeam | undefined;
  labelOf: Map<string, string>;
}) {
  const open = draft.next !== null;
  const describe = (step: DraftStep, team: MatchTeam | undefined) => {
    const who = team?.name ?? (step.number ? `${step.number}. takım` : "sıradaki takım");
    const kind = stepLabel(step.kind);
    if (step.state === "done") {
      const map = (step.mapId && labelOf.get(step.mapId)) ?? "?";
      const off = step.offOrder ? " (sırada bu adım yoktu)" : "";
      return `${kind}: ${who}, ${map}${off}`;
    }
    return `${step.state === "next" ? "Sıradaki" : "Sonra"}: ${kind}, ${who}`;
  };
  // A step made shows its map; the next one its team (or what it is, before the roll is known); one
  // to come only its icon in its team's colour, or the order's figure until the colours are known.
  const content = (step: DraftStep, team: MatchTeam | undefined) => {
    if (step.state === "done") return (step.mapId && labelOf.get(step.mapId)) ?? "?";
    if (step.state === "next") return team?.name ?? stepLabel(step.kind);
    return team ? null : step.number;
  };
  return (
    <div className="pool-draft">
      <ol className="pool-draft-steps" aria-label="Pick-ban sırası">
        {draft.steps.map((step, i) => {
          const team = step.teamId ? teamOf(step.teamId) : undefined;
          const text = describe(step, team);
          return (
            <li
              key={i}
              className={[
                "pool-draft-step",
                `is-${step.state}`,
                `is-${step.kind}`,
                `is-team-${team?.color ?? "none"}`,
                step.offOrder && "is-off-order",
              ]
                .filter(Boolean)
                .join(" ")}
              title={text}
              aria-current={step.state === "next" ? "step" : undefined}
            >
              <Icon
                name={step.kind === "ban" ? "ban" : "target"}
                size={12}
                className="pool-draft-icon"
              />
              {content(step, team) && <span aria-hidden="true">{content(step, team)}</span>}
              <span className="sr-only">{text}</span>
            </li>
          );
        })}
        {draft.alternate && open && (
          <li className="pool-draft-step is-alternate" title="Sonrası sırayla pick">
            <Icon name="swap" size={12} className="pool-draft-icon" />
            <span>sırayla</span>
          </li>
        )}
      </ol>
    </div>
  );
}
