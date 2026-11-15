import type { LiveFinished, LiveRoom, Player, PublicMatch, RefereeRoom, Team } from "../api/types";
import {
  EMPTY_ROOM,
  fromLiveScore,
  fromLiveState,
  sameName,
  SIDE_COLOR,
  SIDE_SLOTS,
  type MapResult,
  type MapScore,
  type RoomState,
  type SlotTeam,
} from "./bancho";
import { matchCode } from "./bracket";
import type { MatchData, PoolEntry } from "./matchPool";

// The room as the referee page and the public match page show it, both from the server's one
// reading of it (LiveRoom): the room's state, its finished maps and the score counted from them.

export interface RosterPlayer<P extends Player = Player> {
  player: P;
  team: Team<P>;
  /** Bracket matches: the colour the player's team plays as (team 1 blue, team 2 red). */
  color: SlotTeam | null;
  /**
   * Bracket matches: the room slot the player belongs in. Team 1's players take 1 and 2, team 2's
   * 3 and 4, in the order the team lists them; the pages list them in that order too.
   */
  home: number | null;
}

export interface TeamTotal {
  teamId: string;
  score: number;
}

/** A finished map of one of the match's rooms, as the server read it. */
export interface FinishedMap {
  /** "<source>:<room>:<n>"; "Sayma" is keyed by it. */
  key: string;
  result: MapResult;
  /** The pool map it was; null outside the pool (a warmup, a mistake), which isn't counted. */
  mapId: string | null;
  /** Each team's summed score on it, best first. */
  totals: TeamTotal[];
  /** Bracket matches: the team that won it, or null for a tie. */
  winnerId: string | null;
  /** It counts towards the score: not left out with "Sayma". */
  counted: boolean;
}

/** A finished map of the pool. */
export interface ResultRow extends FinishedMap {
  entry: PoolEntry;
}

/** A bracket match's map wins, from the room and the referee's hand-awarded results. */
export interface Tally {
  rows: ResultRow[];
  /** Finished maps that aren't in the pool (warmups, mistakes): not shown, not counted. */
  outsidePool: number;
  team1: number;
  team2: number;
}

/**
 * Where the finished maps come from: the rooms (with osu!'s record of them in place of the chat's,
 * once the referee synced it), or, once a bracket match is concluded, the map results saved with
 * its score.
 */
export type ResultSource = "chat" | "saved";

/** "Lobi C" or "W2". */
export function roomLabel(
  room: Pick<RefereeRoom | PublicMatch, "kind" | "lobby_name" | "side" | "number">,
): string {
  if (room.kind === "qualifier") return `Lobi ${room.lobby_name ?? "?"}`;
  return room.side && room.number !== null
    ? matchCode({ side: room.side, number: room.number })
    : "Maç";
}

/** The match's players with their teams and, in a bracket match, the colour and slot each plays in. */
export function rosterOf<P extends Player>(
  room: Pick<MatchData, "kind" | "team1_id" | "team2_id"> & { teams: Team<P>[] },
): RosterPlayer<P>[] {
  const side = (team: Team<P>): 1 | 2 | null =>
    room.kind !== "bracket"
      ? null
      : team.id === room.team1_id
        ? 1
        : team.id === room.team2_id
          ? 2
          : null;
  return room.teams.flatMap((team) => {
    const s = side(team);
    return team.players.map((player, i) => ({
      player,
      team,
      color: s ? SIDE_COLOR[s] : null,
      home: s ? (SIDE_SLOTS[s][i] ?? null) : null,
    }));
  });
}

/** The roster player a score is by: by osu! id when it's known, by name otherwise. */
export function scorer<P extends Player>(
  score: MapScore,
  roster: RosterPlayer<P>[],
): RosterPlayer<P> | undefined {
  return roster.find((p) =>
    score.userId !== null
      ? p.player.osu_id === score.userId
      : sameName(p.player.username, score.username),
  );
}

/** The latest room as BanchoBot has described it. */
export function lobbyOf(live: LiveRoom | null): RoomState {
  return live ? fromLiveState(live.state) : EMPTY_ROOM;
}

function finishedMap(f: LiveFinished): FinishedMap {
  return {
    key: f.key,
    result: {
      beatmapId: f.beatmap_id,
      beatmapName: f.beatmap_name,
      roomMods: f.room_mods,
      freemod: f.freemod,
      scores: f.scores.map(fromLiveScore),
    },
    mapId: f.map_id,
    totals: f.totals.map((t) => ({ teamId: t.team_id, score: t.score })),
    winnerId: f.winner_id,
    counted: f.counted,
  };
}

/** Every room's finished maps, oldest room first. */
export function finishedOf(live: LiveRoom | null): FinishedMap[] {
  return live ? live.finished.map(finishedMap) : [];
}

/** The finished maps of the pool, each with its entry. */
export function poolRows(finished: FinishedMap[], entries: PoolEntry[]): ResultRow[] {
  return finished.flatMap((f) => {
    const entry = entries.find((e) => e.map.id === f.mapId);
    return entry ? [{ ...f, entry }] : [];
  });
}

/** A bracket match's score as the bots count it from its rooms; null without both teams. */
export function liveTally(live: LiveRoom | null, entries: PoolEntry[]): Tally | null {
  if (!live?.tally) return null;
  const finished = finishedOf(live);
  const rows = poolRows(finished, entries);
  return {
    rows,
    outsidePool: finished.length - rows.length,
    team1: live.tally.team1,
    team2: live.tally.team2,
  };
}

/** The pool map the latest room has on. */
export function currentEntryOf(live: LiveRoom | null, entries: PoolEntry[]): PoolEntry | undefined {
  return live?.current_map_id ? entries.find((e) => e.map.id === live.current_map_id) : undefined;
}

/** A match's saved map results as the results list shows them, with its map wins. */
export function savedTally<P extends Player>(
  room: MatchData,
  entries: PoolEntry[],
  roster: RosterPlayer<P>[],
): Tally {
  const rows: ResultRow[] = room.map_results.flatMap((saved, i) => {
    const entry = entries.find((e) => e.map.id === saved.map_id);
    if (!entry) return [];
    const scores: MapScore[] = saved.players.map((p) => {
      const r = roster.find((r) => r.player.id === p.user_id);
      return {
        username: r?.player.username ?? "?",
        userId: r?.player.osu_id ?? null,
        score: p.score,
        passed: p.passed,
        mods: p.mods,
      };
    });
    const totals = [
      { teamId: room.team1_id ?? "", score: saved.team1_score },
      { teamId: room.team2_id ?? "", score: saved.team2_score },
    ].sort((a, b) => b.score - a.score);
    const tied = saved.team1_score === saved.team2_score;
    return [
      {
        key: `saved:${i}`,
        result: {
          beatmapId: entry.map.beatmap_id,
          beatmapName: null,
          roomMods: "",
          freemod: entry.freemod,
          scores,
        },
        mapId: entry.map.id,
        entry,
        totals,
        winnerId: tied ? null : totals[0].teamId,
        counted: true,
      },
    ];
  });
  const wins = (teamId: string | null) =>
    rows.filter((r) => teamId !== null && r.winnerId === teamId).length +
    room.manual_results.filter((m) => m.team_id === teamId).length;
  return { rows, outsidePool: 0, team1: wins(room.team1_id), team2: wins(room.team2_id) };
}
