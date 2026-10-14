import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { setBracketScore } from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import { setMatchLink } from "../../../api/matches";
import { fetchRefereeRoom, fetchRoomChat } from "../../../api/referee";
import type { AdminTeamPlayer, PublicPoolMap, RefereeRoom, RoomChatMessage, Team } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import { conversationKey, isChannel, useIrc, type ChatLine } from "../../../irc/IrcContext";
import {
  applyBancho,
  BANCHO_BOT,
  EMPTY_ROOM,
  ircName,
  modsCommand,
  parseCreated,
  parseJoined,
  playerRef,
  ROOM_PREFIX,
  sameName,
  TEAM_MODE,
  type MapResult,
  type RoomSlot,
  type ScoreMode,
  type SlotTeam,
} from "../../../lib/bancho";
import { matchCode } from "../../../lib/bracket";

export interface RosterPlayer {
  player: AdminTeamPlayer;
  team: Team<AdminTeamPlayer>;
  /** Bracket matches: the colour the player's team plays as (team 1 red, team 2 blue). */
  color: SlotTeam | null;
}

export interface RoomOptions {
  scoreMode: ScoreMode;
  /** Adds NoFail to every non-freemod map. */
  noFail: boolean;
  /** Bracket matches: moves players to their team's colour as they join. */
  autoTeams: boolean;
}

export interface TeamTotal {
  teamId: string;
  score: number;
}

// How long BanchoBot gets to answer !mp make.
const CREATE_TIMEOUT_MS = 20_000;

const PLAY_MODES: Record<string, number> = { osu: 0, taiko: 1, fruits: 2, mania: 3 };

/** "Lobi 3" or "W2". */
export function roomLabel(room: RefereeRoom): string {
  if (room.kind === "qualifier") return `Lobi ${room.lobby_number ?? "?"}`;
  return room.side && room.number !== null ? matchCode({ side: room.side, number: room.number }) : "Maç";
}

/** The osu! room name: "112S: (Team A) vs (Team B)" or "112S: Qualifiers Lobby 3". */
export function roomName(room: RefereeRoom): string {
  if (room.kind === "qualifier") return `${ROOM_PREFIX}: Qualifiers Lobby ${room.lobby_number ?? ""}`.trim();
  const name = (id: string | null) => room.teams.find((t) => t.id === id)?.name ?? "TBD";
  return `${ROOM_PREFIX}: (${name(room.team1_id)}) vs (${name(room.team2_id)})`;
}

function toChatLine(m: RoomChatMessage): ChatLine {
  return { id: m.id, at: Date.parse(m.sent_at), from: m.sender, text: m.text, action: m.action, notice: false };
}

/** Each team's summed score on a map, best first. Players not on the roster don't count. */
export function teamTotals(result: MapResult, roster: RosterPlayer[]): TeamTotal[] {
  const totals = new Map<string, number>();
  for (const s of result.scores) {
    const r = roster.find((p) => sameName(p.player.username, s.username));
    if (r) totals.set(r.team.id, (totals.get(r.team.id) ?? 0) + s.score);
  }
  return [...totals].map(([teamId, score]) => ({ teamId, score })).sort((a, b) => b.score - a.score);
}

/** Owns the referee page's data, the room state read from the chat, and every command it sends. */
export function useRefereeRoom(matchId: string) {
  const user = useSignedInUser();
  const irc = useIrc();
  const [room, setRoom] = useState<RefereeRoom | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [options, setOptions] = useState<RoomOptions>({ scoreMode: 3, noFail: true, autoTeams: true });
  // Finished maps the referee doesn't count (warmups, replays), by index.
  const [ignored, setIgnored] = useState<Set<number>>(new Set());
  // A concluded match's saved room chat, shown instead of the live channel.
  const [archive, setArchive] = useState<ChatLine[] | null>(null);

  const reload = useCallback(async () => {
    try {
      setRoom(await fetchRefereeRoom(matchId));
      setLoadError(null);
    } catch (e) {
      setLoadError(errorMessage(e));
    }
  }, [matchId]);

  useEffect(() => {
    setRoom(null);
    setIgnored(new Set());
    reload();
  }, [reload]);

  const concluded = room !== null && room.concluded_at !== null;
  const roomId = room?.mp_id ?? null;
  useEffect(() => {
    setArchive(null);
    if (!concluded || roomId === null) return;
    let current = true;
    fetchRoomChat(matchId)
      .then((chat) => {
        if (current) setArchive(chat.messages.map(toChatLine));
      })
      .catch((e) => current && setError(errorMessage(e)));
    return () => {
      current = false;
    };
  }, [matchId, concluded, roomId]);

  const canRun = room !== null && (hasRole(user, "host") || room.referee?.id === user.id);
  const channel = room?.mp_id ? `#mp_${room.mp_id}` : null;
  const conversation = channel ? irc.conversations[conversationKey(channel)] : undefined;
  const online = irc.status === "online";
  const inChannel = online && conversation?.joined === true;

  // The lobby as BanchoBot has described it: in this session, or in the saved chat
  // once the match is concluded.
  const lines = concluded ? archive : conversation?.lines;
  const lobby = useMemo(
    () => (lines ?? []).filter((l) => l.from === BANCHO_BOT && !l.notice).reduce((r, l) => applyBancho(r, l.text), EMPTY_ROOM),
    [lines],
  );

  const roster = useMemo<RosterPlayer[]>(() => {
    if (!room) return [];
    const color = (team: Team<AdminTeamPlayer>): SlotTeam | null =>
      room.kind !== "bracket" ? null : team.id === room.team1_id ? "red" : team.id === room.team2_id ? "blue" : null;
    return room.teams.flatMap((team) => team.players.map((player) => ({ player, team, color: color(team) })));
  }, [room]);

  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find((r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username)),
    [roster],
  );

  const say = useCallback(
    (text: string, target: string | null = channel) => {
      if (target) irc.send(target, text);
    },
    [channel, irc],
  );

  // Joins the room's channel once per connection, so a reconnect gets back in. After
  // that it's up to the referee: a closed room's channel stays closed.
  const { join, status } = irc;
  const autoJoined = useRef(new Set<string>());
  useEffect(() => {
    if (status !== "online") {
      autoJoined.current.clear();
      return;
    }
    if (!channel || concluded || autoJoined.current.has(channel)) return;
    autoJoined.current.add(channel);
    if (!conversation?.joined) join(channel);
  }, [status, channel, concluded, conversation?.joined, join]);

  // Puts bracket players on their team's colour as they join.
  const autoTeams = room?.kind === "bracket" && canRun && options.autoTeams;
  const autoRef = useRef({ autoTeams, roster, channel, say });
  autoRef.current = { autoTeams, roster, channel, say };
  const { onMessage } = irc;
  useEffect(
    () =>
      onMessage((msg) => {
        const { autoTeams, roster, channel, say } = autoRef.current;
        if (!autoTeams || msg.from !== BANCHO_BOT || !channel || !sameName(msg.target, channel)) return;
        const joined = parseJoined(msg.text);
        const r = joined && roster.find((p) => sameName(p.player.username, joined.username));
        if (joined && r?.color && joined.team !== r.color) say(`!mp team ${playerRef(r.player.osu_id)} ${r.color}`);
      }),
    [onMessage],
  );

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  // Team mode, score mode and size; locks the slots, then invites everyone.
  function setUp(target: string) {
    if (!room) return;
    const players = roster.map((r) => r.player);
    const mode = room.kind === "bracket" ? TEAM_MODE.teamVs : TEAM_MODE.headToHead;
    const size = Math.min(16, Math.max(2, players.length));
    say(`!mp set ${mode} ${options.scoreMode} ${size}`, target);
    say("!mp lock", target);
    for (const p of players) say(`!mp invite ${playerRef(p.osu_id)}`, target);
    say("!mp settings", target);
  }

  // Asks BanchoBot for a tournament room and waits for the room id in its reply.
  function makeRoom(name: string): Promise<number> {
    return new Promise((resolve, reject) => {
      const off = onMessage((msg) => {
        if (msg.from !== BANCHO_BOT || isChannel(msg.target)) return;
        const created = parseCreated(msg.text);
        if (created) {
          done();
          resolve(created.mpId);
        } else if (/cannot create/i.test(msg.text)) {
          done();
          reject(new Error(msg.text));
        }
      });
      const timer = window.setTimeout(() => {
        done();
        reject(new Error("BanchoBot cevap vermedi. Sohbette ne dediğine bak."));
      }, CREATE_TIMEOUT_MS);
      function done() {
        off();
        window.clearTimeout(timer);
      }
      irc.send(BANCHO_BOT, `!mp make ${name}`);
    });
  }

  const actions = {
    reload,
    setOptions,
    say,

    /** !mp make, saves the room link to the match, then sets the room up. */
    createRoom: () =>
      run(async () => {
        if (!room) return;
        const mpId = await makeRoom(roomName(room));
        setUp(`#mp_${mpId}`);
        await setMatchLink(room.id, String(mpId));
        await reload();
      }),

    /** Points the match at a room made some other way. */
    linkRoom: (link: string) =>
      run(async () => {
        if (!room) return;
        await setMatchLink(room.id, link);
        await reload();
      }),

    setUpRoom: () => channel && setUp(channel),
    invite: (osuId: number) => say(`!mp invite ${playerRef(osuId)}`),
    inviteAll: () => roster.forEach((r) => say(`!mp invite ${playerRef(r.player.osu_id)}`)),
    moveToTeam: (slot: Pick<RoomSlot, "userId" | "username">, team: SlotTeam) =>
      say(`!mp team ${slot.userId ? playerRef(slot.userId) : ircName(slot.username)} ${team}`),

    /** Puts everyone in the room on their team's colour. */
    fixTeams: () => {
      for (const slot of lobby.slots) {
        const r = findRoster(slot);
        if (r?.color && slot.team !== r.color) say(`!mp team ${playerRef(r.player.osu_id)} ${r.color}`);
      }
    },

    pickMap: (map: PublicPoolMap, freemod: boolean) => {
      say(`!mp map ${map.beatmap_id} ${PLAY_MODES[map.mode] ?? 0}`);
      say(modsCommand(map.mods, freemod, options.noFail));
    },

    settings: () => say("!mp settings"),
    lock: (locked: boolean) => say(locked ? "!mp lock" : "!mp unlock"),
    timer: (seconds: number) => say(`!mp timer ${seconds}`),
    abortTimer: () => say("!mp aborttimer"),
    start: (seconds: number) => say(seconds > 0 ? `!mp start ${seconds}` : "!mp start"),
    abort: () => say("!mp abort"),
    close: () => say("!mp close"),
    rejoin: () => channel && join(channel),

    toggleIgnored: (index: number) =>
      setIgnored((prev) => {
        const next = new Set(prev);
        if (!next.delete(index)) next.add(index);
        return next;
      }),

    saveScore: (team1: number, team2: number) =>
      run(async () => {
        if (!room) return;
        await setBracketScore(room.id, team1, team2);
        await reload();
      }),
  };

  return {
    room,
    archive,
    loadError,
    error,
    busy,
    canRun,
    channel,
    online,
    inChannel,
    lobby,
    roster,
    findRoster,
    options,
    ignored,
    actions,
  };
}

export type RefereeRoomState = ReturnType<typeof useRefereeRoom>;
