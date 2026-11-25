import { useCallback, useMemo } from "react";
import { fetchPublicChat, fetchPublicMatch } from "../api/matches";
import type { Player } from "../api/types";
import { sameName, type RoomSlot } from "./bancho";
import { standingOf, type Standing } from "./bestOf";
import {
  currentEntryOf,
  finishedOf,
  lobbyOf,
  rosterOf,
  savedTally,
  type FinishedMap,
  type RosterPlayer,
  type Tally,
} from "./liveRoom";
import { draftTurn, matchDraft, matchTeams, poolEntries, type MatchTeam } from "./matchPool";
import { toChatLine, type ChatLine } from "./roomChat";
import { useMatchFeed } from "./useMatchFeed";
import { useLive } from "../live/LiveContext";

// The public match page reads the same room the referee page runs: the match as saved, the room
// as its bot reads it (the same reading on every page), and each room's chat as the bot saved it,
// then every change over the live socket (lib/useMatchFeed.ts, which the referee page follows its
// match with too).

/** One of the match's rooms with its chat so far. */
export interface PublicRoomChat {
  mpId: number;
  url: string;
  lines: ChatLine[];
  /** The match's latest room, where it's being played. */
  current: boolean;
}

// How often the page reloads by itself while the live socket is down and the match is on.
const FALLBACK_MS = 15_000;

/**
 * `spectate: false` follows the match without counting among its spectators (the broadcast
 * overlays, which aren't people watching).
 */
export function useMatchPage(matchId: string, { spectate = true }: { spectate?: boolean } = {}) {
  const { spectators: watchers } = useLive();
  // Live: it has a room and isn't concluded. Only then is it followed over the socket. A concluded
  // match's chat is the saved one; an upcoming one is followed once its room is added, which
  // reloads the match.
  const {
    match,
    live,
    loadError,
    chat,
    chatLoaded,
    connected,
    following: isLive,
  } = useMatchFeed(matchId, {
    fetchMatch: fetchPublicMatch,
    fetchChat: fetchPublicChat,
    follows: (m) => m.concluded_at === null && m.rooms.length > 0,
    spectate,
    staff: false,
    pollMs: FALLBACK_MS,
    pollAlways: false,
  });

  const roomChats = useMemo<PublicRoomChat[]>(
    () =>
      (match?.rooms ?? []).map((r, i, all) => ({
        mpId: r.mp_id,
        url: r.url,
        lines: (chat.get(r.mp_id) ?? []).map(toChatLine),
        current: i === all.length - 1,
      })),
    [match, chat],
  );

  const entries = useMemo(() => (match ? poolEntries(match) : []), [match]);
  const lobby = useMemo(() => lobbyOf(live), [live]);
  const finished = useMemo<FinishedMap[]>(() => finishedOf(live), [live]);
  const currentEntry = useMemo(() => currentEntryOf(live, entries), [live, entries]);

  const roster = useMemo<RosterPlayer<Player>[]>(() => (match ? rosterOf(match) : []), [match]);
  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find(
        (r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username),
      ),
    [roster],
  );
  const teams = useMemo<MatchTeam[]>(() => (match ? matchTeams(match) : []), [match]);

  // A bracket match's sheet is what the site saved: the counted maps with every player's score,
  // as Discord shows it (the room's bot saves it as each map finishes). Its score is the saved
  // one, which a host may have entered by hand.
  const tally = useMemo<Tally | null>(() => {
    if (!match || match.kind !== "bracket") return null;
    const saved = savedTally(match, entries, roster);
    return {
      ...saved,
      team1: match.team1_score ?? saved.team1,
      team2: match.team2_score ?? saved.team2,
    };
  }, [match, entries, roster]);

  // Where the saved score stands against the stage's best of (the tiebreaker, a decided match).
  const bestOf = match?.kind === "bracket" ? (match.stage?.best_of ?? null) : null;
  const standing = useMemo<Standing | null>(
    () =>
      bestOf !== null && tally && !match?.forfeit_slot && match?.concluded_at === null
        ? standingOf(bestOf, tally.team1, tally.team2)
        : null,
    [bestOf, tally, match?.forfeit_slot, match?.concluded_at],
  );

  // The pool maps played so far, from the saved results and the rooms.
  const played = useMemo(
    () =>
      new Set<string | undefined>([
        ...(match?.map_results ?? []).map((r) => r.map_id ?? undefined),
        ...finished.map((f) => f.mapId ?? undefined),
      ]),
    [match, finished],
  );

  // Who is watching with this tab, only while the match is live (that's when it's watched).
  const spectators = isLive && watchers?.match_id === matchId ? watchers : null;
  // The bans and picks against the stage's order; the next one only while the match is live and
  // there's one to make.
  const drafting = isLive && standing?.kind !== "tiebreaker" && standing?.kind !== "won";
  const draft = useMemo(
    () => (match ? matchDraft(match, match.next_pick_team_id, drafting) : null),
    [match, drafting],
  );
  const turn = draftTurn(draft, teams);

  return {
    match,
    loadError,
    connected,
    roomChats,
    chatLoaded,
    lobby,
    finished,
    currentEntry,
    mapStartedAt: lobby.mapStartedAt,
    roster,
    findRoster,
    teams,
    tally,
    played,
    live: isLive,
    /** The bots' latest reading of the room, kept current over the socket (`match.live` is only
     * the one the match was loaded with). */
    liveRoom: live,
    draft,
    turn,
    standing,
    spectators,
  };
}

export type MatchPageState = ReturnType<typeof useMatchPage>;
