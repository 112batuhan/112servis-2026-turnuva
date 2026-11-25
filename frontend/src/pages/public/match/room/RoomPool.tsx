import { useMemo, type CSSProperties } from "react";
import { DraftStrip } from "../../../../components/match/PoolView";
import type { StepKind } from "../../../../lib/draftOrder";
import { poolEntries, type MatchTeam, type PoolEntry } from "../../../../lib/matchPool";
import { draftRefusal, HOLD_MS } from "../../../../lib/playerRoom";
import { useHold } from "../../../../lib/useHold";
import type { MatchPageState } from "../../../../lib/useMatchPage";
import type { RoomMe } from "./PlayerRoomPage";
import "../../../../components/match/PoolView.css";
import "./RoomPool.css";

interface Props {
  page: MatchPageState;
  me: RoomMe;
  /** The viewer's team's ban or pick, while it's theirs to make on the auto referee's clock. */
  turn: StepKind | null;
  /** The map whose ban or pick was just sent. */
  pending: string | null;
  onChoose: (entry: PoolEntry, kind: StepKind) => void;
  /** A hold on the map began (the bot waits for it if the turn runs out meanwhile). */
  onHold: (entry: PoolEntry) => void;
}

/**
 * The match's pool as the players play it: every map over its cover, by category, with what became
 * of it. On the team's turn every map it may take is held for three seconds to ban or pick it; the
 * ones it may not say why.
 */
export function RoomPool({ page, me, turn, pending, onChoose, onHold }: Props) {
  const { match, draft, played, currentEntry, teams } = page;
  const entries = useMemo(() => (match ? poolEntries(match) : []), [match]);
  if (!match) return null;

  const teamOf = (id: string) => teams.find((t) => t.id === id);
  // The draft strip names the viewer's team "Siz", as the tiles do, so a long name never hides
  // whose step it is.
  const draftTeamOf = (id: string) => {
    const t = teamOf(id);
    return t && t.id === me.teamId ? { ...t, name: "Siz" } : t;
  };
  const labelOf = new Map(entries.map((e) => [e.map.id, e.label]));
  const groups: { abbreviation: string; name: string; color: string; entries: PoolEntry[] }[] = [];
  for (const e of entries) {
    const last = groups[groups.length - 1];
    if (last?.abbreviation === e.abbreviation) last.entries.push(e);
    else {
      const name = match.categories.find((c) => c.abbreviation === e.abbreviation)?.name ?? "";
      groups.push({ abbreviation: e.abbreviation, name, color: e.color, entries: [e] });
    }
  }
  const live = !match.concluded_at;

  return (
    <section className="room-pool" aria-label="Mappool">
      <header className="room-pool-head">
        <h2 className="room-pool-title">Mappool</h2>
        {draft && draft.steps.length > 0 && entries.length > 0 && (
          <DraftStrip draft={draft} teamOf={draftTeamOf} labelOf={labelOf} takeBack={undefined} />
        )}
      </header>

      {!match.stage && <p className="room-pool-note">Bu maça stage seçilmemiş.</p>}
      {match.stage && entries.length === 0 && (
        <p className="room-pool-note">
          {match.stage.published
            ? "Bu stage'in havuzunda map yok."
            : "Bu stage'in havuzu henüz yayımlanmadı; yayımlanınca burada görünür."}
        </p>
      )}

      <div className="room-pool-scroll">
        {groups.map((g) => (
          <section
            key={g.abbreviation}
            className="room-pool-group"
            style={{ "--cat": g.color } as CSSProperties}
            aria-label={g.name || g.abbreviation}
          >
            <ul className="room-pool-grid">
              {g.entries.map((entry) => {
                const id = entry.map.id;
                const ban = match.bans.find((b) => b.map_id === id);
                const pick = match.picks.find((p) => p.map_id === id);
                const refusal = turn
                  ? draftRefusal(entry, turn, me.teamId, match, played, entries)
                  : null;
                return (
                  <li key={id}>
                    <MapTile
                      entry={entry}
                      bannedBy={ban ? teamOf(ban.team_id) : undefined}
                      pickedBy={pick ? teamOf(pick.team_id) : undefined}
                      played={played.has(id)}
                      onRoom={live && currentEntry?.map.id === id}
                      turn={turn}
                      refusal={refusal}
                      pending={pending === id}
                      locked={pending !== null}
                      onChoose={onChoose}
                      onHold={onHold}
                    />
                  </li>
                );
              })}
            </ul>
          </section>
        ))}
      </div>
    </section>
  );
}

interface TileProps {
  entry: PoolEntry;
  bannedBy: MatchTeam | undefined;
  pickedBy: MatchTeam | undefined;
  played: boolean;
  onRoom: boolean;
  turn: StepKind | null;
  refusal: string | null;
  pending: boolean;
  /** Another map is being sent. */
  locked: boolean;
  onChoose: (entry: PoolEntry, kind: StepKind) => void;
  onHold: (entry: PoolEntry) => void;
}

function MapTile({
  entry,
  bannedBy,
  pickedBy,
  played,
  onRoom,
  turn,
  refusal,
  pending,
  locked,
  onChoose,
  onHold,
}: TileProps) {
  const { map, label } = entry;
  const armed = turn !== null && refusal === null && !locked;
  const { holding, left, bind } = useHold(
    HOLD_MS,
    () => turn && onChoose(entry, turn),
    armed,
    () => onHold(entry),
  );
  const verb = turn === "ban" ? "Banla" : "Seç";
  const state = bannedBy ? "is-banned" : pickedBy || played ? "is-played" : "";
  const classes = [
    "room-map",
    state,
    onRoom && "is-on",
    armed && `is-armed is-${turn}`,
    holding && "is-holding",
    pending && "is-pending",
    turn && refusal && "is-refused",
  ]
    .filter(Boolean)
    .join(" ");

  const status = holding ? (
    <span className="room-map-state is-action">
      {turn === "ban" ? "Banlanıyor…" : "Seçiliyor…"}
    </span>
  ) : (
    tileState({
      bannedBy,
      pickedBy,
      played,
      onRoom,
      armed,
      refusal: turn ? refusal : null,
      pending,
    })
  );

  return (
    <article
      className={classes}
      style={
        {
          "--cover": map.cover_url ? `url("${map.cover_url}")` : "none",
          "--progress": holding ? 1 - left / HOLD_MS : 0,
        } as CSSProperties
      }
      aria-label={`${label}: ${map.artist} - ${map.title} [${map.version}]`}
    >
      <div className="room-map-cover" aria-hidden="true" />
      <div className="room-map-fill" aria-hidden="true" />
      <div className="room-map-body">
        <span className="room-map-label num">{label}</span>
        <span className="room-map-text">
          <span className="room-map-title">{map.title}</span>
          {status ?? <span className="room-map-version">{map.version}</span>}
        </span>
      </div>
      {armed && (
        <button
          type="button"
          className="room-map-hold"
          aria-label={`${label} ${verb.toLowerCase()}: 3 saniye basılı tut`}
          aria-pressed={holding}
          {...bind}
        />
      )}
      {holding && (
        <span className="room-map-count num" aria-hidden="true">
          {Math.ceil(left / 1000)}
        </span>
      )}
    </article>
  );
}

// What became of the map, or what the viewer can do with it now, as one line of text; null when
// there's nothing to say.
function tileState({
  bannedBy,
  pickedBy,
  played,
  onRoom,
  armed,
  refusal,
  pending,
}: {
  bannedBy: MatchTeam | undefined;
  pickedBy: MatchTeam | undefined;
  played: boolean;
  onRoom: boolean;
  armed: boolean;
  refusal: string | null;
  pending: boolean;
}) {
  if (pending) return <span className="room-map-state is-action">Gönderiliyor…</span>;
  if (bannedBy) {
    return (
      <span className={`room-map-state is-team-${bannedBy.color}`}>{bannedBy.name} banladı</span>
    );
  }
  // A pick says only who picked it, in their side's colour.
  if (pickedBy) {
    return <span className={`room-map-state is-team-${pickedBy.color}`}>{pickedBy.name}</span>;
  }
  if (onRoom) return <span className="room-map-state is-on">Odada</span>;
  if (played) return <span className="room-map-state">Oynandı</span>;
  // An armed tile says nothing more: its edge and the turn bar say it can be held.
  if (armed) return null;
  if (refusal) return <span className="room-map-state is-refusal">{refusal}</span>;
  return null;
}
