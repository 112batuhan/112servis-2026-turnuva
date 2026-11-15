import { IconButton } from "../../../components/IconButton";
import { matchTeams, type MatchTeam, type PoolEntry } from "../../../lib/matchPool";
import { botBlocker, type RefereeRoomState } from "./useRefereeRoom";
import { PoolView, draftPhase } from "../../../components/match/PoolView";

/**
 * The match's map pool. Clicking a map has the room's bot set it and its mods in the room. In a
 * bracket match the referee also marks the bans and picks as the stage's order has them: each map
 * offers the next step's buttons (ban or pick) for both teams, the one whose turn it is first; a
 * pick sets the map too. Banned maps can't be picked.
 */
export function PoolPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, currentEntry, finished, botReady, canRun, busy, actions, race, draft, turn } =
    referee;
  if (!room) return null;
  // The server tells apart a beatmap that's in the pool twice by the mods it was played with.
  const played = new Set<string | undefined>(finished.map((f) => f.mapId ?? undefined));
  const blocker = botBlocker(referee);
  const teams = matchTeams(room);
  const canMark = teams.length === 2 && canRun && room.concluded_at === null;
  // Once the order has run out (or at the tiebreaker) only picks are left to record.
  const banning = turn?.kind === "ban";
  // The team whose turn it is leads; the other's button stays, quieter, for a roll the order didn't foresee.
  const offTurn = (team: MatchTeam) =>
    turn?.team && turn.team.id !== team.id ? " is-off-turn" : "";

  // A ban or pick to take back, or the buttons for whichever phase the match is in.
  function markActions(
    entry: PoolEntry,
    banned: MatchTeam | undefined,
    picked: MatchTeam | undefined,
  ) {
    const { label, map, tiebreaker } = entry;
    if (banned) {
      return (
        <IconButton
          icon="undo"
          label={`${label} yasağını kaldır`}
          disabled={busy}
          onClick={() => actions.unban(map.id)}
        />
      );
    }
    if (picked) {
      return (
        <IconButton
          icon="undo"
          label={`${label} seçimini kaldır`}
          disabled={busy}
          onClick={() => actions.unpick(map.id)}
        />
      );
    }
    if (banning) {
      return teams.map((team) => (
        <IconButton
          key={team.id}
          icon="ban"
          className={`pool-mark-button is-team-${team.color}${offTurn(team)}`}
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
        className={`pool-mark-button is-team-${team.color}${offTurn(team)}`}
        label={`${label}: ${team.name} seçti`}
        disabled={busy}
        onClick={() => actions.pick(map, team.id)}
      />
    ));
  }

  return (
    <PoolView
      room={room}
      currentEntry={currentEntry}
      played={played}
      draft={draft}
      phase={canMark ? draftPhase(turn, race) : null}
      pick={{
        disabled: (_entry, banned) => !botReady || banned !== undefined,
        title: (entry, banned) =>
          banned
            ? `${entry.label}: ${banned.name} yasakladı`
            : botReady
              ? `${entry.label} seç: !mp map ${entry.map.beatmap_id}, ${entry.freemod ? "Freemod" : entry.map.mods || "NM"}`
              : (blocker ?? ""),
        onPick: (entry) => actions.pickMap(entry.map),
      }}
      renderActions={canMark ? markActions : undefined}
    />
  );
}
