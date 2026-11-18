import { useCallback, useMemo, useState } from "react";
import {
  addManualResult,
  forfeitMatch,
  removeManualResult,
  removeMapBan,
  removeMapPick,
  removeMissedTurn,
  setMapBan,
  setMapPick,
} from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import { addMatchRoom, concludeMatch, removeMatchRoom } from "../../../api/matches";
import {
  createRoom,
  fetchRefereeRoom,
  fetchRoomChat,
  roomCommand,
  saveRoomScore,
  setAutoref as saveAutoref,
  setIgnoredMap,
  setNextPick as saveNextPick,
  setRoomOptions,
  syncFromOsu,
} from "../../../api/referee";
import type {
  AdminTeamPlayer,
  PublicPoolMap,
  RefereeRoom,
  RoomAccess,
  RoomCommand,
} from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import {
  isRoomRef,
  sameName,
  type RoomSlot,
  type ScoreMode,
  type SlotTeam,
} from "../../../lib/bancho";
import { standingOf, type Standing } from "../../../lib/bestOf";
import { LIVE_REFRESH_MS } from "../../../lib/useLiveRefresh";
import {
  currentEntryOf,
  finishedOf,
  liveTally,
  lobbyOf,
  rosterOf,
  savedTally,
  type FinishedMap,
  type ResultSource,
  type RosterPlayer,
  type Tally,
} from "../../../lib/liveRoom";
import {
  draftTurn,
  matchDraft,
  matchTeams,
  poolEntries,
  type MatchTeam,
} from "../../../lib/matchPool";
import { toChatLine, type ChatLine } from "../../../lib/roomChat";
import { useRoomStaff } from "../../../lib/roomStaff";
import { useAction } from "../../../lib/useAction";
import { useMatchFeed } from "../../../lib/useMatchFeed";
import { useAutorefLog } from "./useAutorefLog";

export interface RoomOptions {
  scoreMode: ScoreMode;
  /** Adds NoFail to every non-freemod map. */
  noFail: boolean;
  /** Bracket matches: the bot moves players to their slot and their team's colour as they join. */
  autoPlace: boolean;
  /** The bot kicks anyone who joins without being a player of the match, a host, a referee or a streamer. */
  autoKick: boolean;
}

/** One of the match's rooms in the chat: what its bot saved, then what it saves as it comes. */
export interface RoomChatView {
  mpId: number;
  /** "#mp_<id>" */
  channel: string;
  url: string;
  lines: ChatLine[];
  /** The match's latest room, where it's being played. */
  current: boolean;
  /** The bot that runs it, by osu! username. */
  bot: string | null;
  access: RoomAccess;
}

const DEFAULT_OPTIONS: RoomOptions = {
  scoreMode: 3,
  noFail: true,
  autoPlace: true,
  autoKick: true,
};

/**
 * Owns the referee page's data. The room itself (its slots, map, finished maps, score) is the
 * bots' reading of it, the same on every page and kept up to date over the live socket; every
 * command, and every line the referee writes in the room, goes to the room's bot.
 */
export function useRefereeRoom(matchId: string) {
  const user = useSignedInUser();
  const { busy, error, setError, run } = useAction();

  // The page follows the match over the live socket: its room as the bots read it and every line
  // its bot saves, without counting among its spectators. Whatever any referee or host changes
  // (bans, picks, results, "Sayma", the options, the rooms) reloads it here; while the socket is
  // down, the page polls instead.
  const {
    match: room,
    setMatch: setRoom,
    live,
    loadError,
    chat,
    reload,
  } = useMatchFeed(matchId, {
    fetchMatch: fetchRefereeRoom,
    fetchChat: fetchRoomChat,
    follows: (r) => r.rooms.length > 0,
    spectate: false,
    staff: true,
    pollMs: LIVE_REFRESH_MS,
    pollAlways: true,
    onChatError: setError,
  });
  const rooms = useMemo(() => room?.rooms ?? [], [room]);

  const concluded = room !== null && room.concluded_at !== null;
  const current = rooms.length > 0 ? rooms[rooms.length - 1] : null;
  const channel = current ? `#mp_${current.mp_id}` : null;
  // Any referee (or host) can run any match, so one can step in when its own referee can't.
  const canRun = room !== null && hasRole(user, "referee");

  const badges = live?.rooms;
  const roomChats = useMemo<RoomChatView[]>(
    () =>
      rooms.map((r, i) => {
        const badge = badges?.find((b) => b.mp_id === r.mp_id);
        const channel = `#mp_${r.mp_id}`;
        return {
          mpId: r.mp_id,
          channel,
          url: r.url,
          lines: (chat.get(r.mp_id) ?? []).map(toChatLine),
          current: i === rooms.length - 1,
          bot: badge?.bot ?? null,
          access: badge?.access ?? "no_bot",
        };
      }),
    [rooms, badges, chat],
  );
  const latestChat = roomChats.length > 0 ? roomChats[roomChats.length - 1] : null;
  // Where the latest room's bot stands: only a bot in the room can run it.
  const access: RoomAccess | null = latestChat?.access ?? null;
  const botName = latestChat?.bot ?? null;
  const botReady = canRun && !concluded && access === "in";

  // What the room keeps about the match, shared by everyone on it (RoomSettings).
  const settings = room?.settings;
  const options = useMemo<RoomOptions>(
    () =>
      settings
        ? {
            scoreMode: settings.score_mode as ScoreMode,
            noFail: settings.no_fail,
            autoPlace: settings.auto_place,
            autoKick: settings.auto_kick,
          }
        : DEFAULT_OPTIONS,
    [settings],
  );
  // The room's bot refereeing by itself, and where it stands (null until the bots run the match).
  const autoEnabled = settings?.auto_ref ?? true;
  // While the auto referee runs the match, the controls that referee it wait for manual mode, so a
  // click can't fight the bot. Talking to the players, opening a room and the switch stay open.
  const autoLock = autoEnabled && !concluded ? AUTO_LOCK : null;
  const canControl = canRun && autoLock === null;
  const controlsReady = botReady && autoLock === null;
  const autoref = live?.autoref ?? null;
  const autorefLog = useAutorefLog(matchId, rooms.map((r) => r.mp_id).join(","));

  const ignoredMaps = room?.ignored_maps;
  const ignored = useMemo(() => new Set(ignoredMaps ?? []), [ignoredMaps]);

  // Changes what the room keeps at once and saves it; if saving fails, the page goes back to
  // what the server has.
  const keep = useCallback(
    (change: (r: RefereeRoom) => RefereeRoom, save: () => Promise<unknown>) => {
      setRoom((r) => r && change(r));
      save().catch((e) => {
        setError(errorMessage(e));
        reload();
      });
    },
    [reload, setError, setRoom],
  );

  const entries = useMemo(() => (room ? poolEntries(room) : []), [room]);
  const lobby = useMemo(() => lobbyOf(live), [live]);
  const currentEntry = useMemo(() => currentEntryOf(live, entries), [live, entries]);
  const finished = useMemo<FinishedMap[]>(() => finishedOf(live), [live]);
  const roster = useMemo<RosterPlayer<AdminTeamPlayer>[]>(
    () => (room ? rosterOf(room) : []),
    [room],
  );
  const staff = useRoomStaff();

  const findRoster = useCallback(
    (slot: Pick<RoomSlot, "username" | "userId">) =>
      roster.find(
        (r) => r.player.osu_id === slot.userId || sameName(r.player.username, slot.username),
      ),
    [roster],
  );

  // A concluded match's result stands: its map results as they were saved with the score (the
  // record the Discord results post is made from).
  const fromSaved = concluded && room?.kind === "bracket" && room.map_results.length > 0;
  const source: ResultSource = fromSaved ? "saved" : "chat";
  const tally = useMemo<Tally | null>(() => {
    if (!room || room.kind !== "bracket") return null;
    return fromSaved ? savedTally(room, entries, roster) : liveTally(live, entries);
  }, [room, live, entries, roster, fromSaved]);
  const scoreSaved = live?.score_saved ?? false;

  // How many maps the match is played to and where the score stands against it. Concluding stays
  // the referee's call: a decided match only offers it.
  const bestOf = room?.kind === "bracket" ? (room.stage?.best_of ?? null) : null;
  const standing = useMemo<Standing | null>(
    () =>
      bestOf !== null && tally !== null && !room?.forfeit_slot
        ? standingOf(bestOf, tally.team1, tally.team2)
        : null,
    [bestOf, tally, room?.forfeit_slot],
  );
  const teams = useMemo<MatchTeam[]>(() => (room ? matchTeams(room) : []), [room]);
  // The bans and picks against the stage's order, and the next one (the order's, or the referee's
  // say until the next mark). Nobody bans or picks at the tiebreaker or once a team has won.
  const drafting =
    room?.concluded_at === null && standing?.kind !== "tiebreaker" && standing?.kind !== "won";
  const draft = useMemo(
    () => (room ? matchDraft(room, room.settings.next_pick_team_id, drafting) : null),
    [room, drafting],
  );
  const turn = draftTurn(draft, teams);

  const streamer = room?.streamer ?? null;
  const streamerIsRef = streamer !== null && isRoomRef(lobby, streamer.username);
  const [syncing, setSyncing] = useState(false);

  // A command for the room's bot. It's queued at once; what it does shows as BanchoBot answers.
  const command = useCallback(
    (cmd: RoomCommand) => {
      setError(null);
      roomCommand(matchId, cmd).catch((e) => setError(errorMessage(e)));
    },
    [matchId, setError],
  );

  const actions = {
    reload,

    /** Says something in the room through its bot; resolves to whether it went out. */
    say: (text: string) => {
      // Chat stays open in auto mode; an !mp command would referee the match behind the bot.
      if (autoLock !== null && /^!mp\b/i.test(text.trim())) {
        setError(`${autoLock}: !mp komutları manuel kontrolde gönderilir.`);
        return Promise.resolve(false);
      }
      return roomCommand(matchId, { type: "say", text }).then(
        () => true,
        (e) => {
          setError(errorMessage(e));
          return false;
        },
      );
    },

    /** Changes the room options for everyone on the page. */
    setOptions: (next: RoomOptions) => {
      if (!room) return;
      const saved = {
        score_mode: next.scoreMode,
        no_fail: next.noFail,
        auto_place: next.autoPlace,
        auto_kick: next.autoKick,
      };
      keep(
        (r) => ({ ...r, settings: { ...r.settings, ...saved } }),
        () => setRoomOptions(room.id, saved),
      );
    },

    /**
     * Switches the match between the auto referee and manual control for everyone on it. The old
     * controls work either way; back on auto, the bot continues where it left off.
     */
    setAutoref: (enabled: boolean) => {
      if (!room) return;
      keep(
        (r) => ({ ...r, settings: { ...r.settings, auto_ref: enabled } }),
        () => saveAutoref(room.id, enabled),
      );
    },

    /** The least busy bot makes a room, sets it up and adds it to the match. Also replaces a broken room. */
    createRoom: () =>
      run(async () => {
        await createRoom(matchId);
        await reload();
      }),

    /** Adds a room made some other way; it's handed to a bot, which joins once it's added as a referee. */
    addRoom: (link: string) =>
      run(async () => {
        await addMatchRoom(matchId, link);
        await reload();
      }),

    removeRoom: (mpId: number) =>
      run(async () => {
        await removeMatchRoom(matchId, mpId);
        await reload();
      }),

    setUpRoom: () => command({ type: "set_up" }),
    invite: (osuId: number) => command({ type: "invite", osu_id: osuId }),
    inviteAll: () => command({ type: "invite_all" }),
    /** Moves whoever is in a slot to slot `to` and onto `team` (either null to leave it), swapping with whoever holds it. */
    place: (slot: RoomSlot, to: number | null, team: SlotTeam | null) =>
      command({ type: "move", slot: slot.slot, to, team }),
    /** Puts every bracket player in the room in their slot and on their team's colour. */
    fixPlacement: () => command({ type: "place_all" }),
    kick: (slot: RoomSlot) => command({ type: "kick", slot: slot.slot }),
    /** Removes everyone in the room who isn't a player of the match, a host, a referee or a streamer. */
    kickOutsiders: () => command({ type: "kick_outsiders" }),
    /** Makes someone one of the room's referees, or takes it back. */
    addRef: (username: string) => command({ type: "add_ref", username }),
    removeRef: (username: string) => command({ type: "remove_ref", username }),
    /** The match's streamer as one of the room's referees (their tournament client shows its chat), or back. */
    toggleStreamerRef: () => {
      if (!streamer) return;
      command(
        streamerIsRef
          ? { type: "remove_ref", username: streamer.username }
          : { type: "add_ref", username: streamer.username },
      );
    },
    /** Sets a map and its mods in the room, without recording who picked it. */
    pickMap: (map: PublicPoolMap) => command({ type: "map", map_id: map.id }),

    /** Records a team's pick, then sets the map in the room. */
    pick: (map: PublicPoolMap, teamId: string) =>
      run(async () => {
        await setMapPick(matchId, map.id, teamId);
        if (botReady) await roomCommand(matchId, { type: "map", map_id: map.id });
        await reload();
      }),

    unpick: (mapId: string) =>
      run(async () => {
        await removeMapPick(matchId, mapId);
        await reload();
      }),

    /** Takes back a ban or pick that ran out on the auto referee's clock. */
    unmiss: (missedId: string) =>
      run(async () => {
        await removeMissedTurn(matchId, missedId);
        await reload();
      }),

    /** Says who goes next, overriding the stage's order until the next ban or pick. */
    setNextPick: (teamId: string) => {
      if (!room) return;
      keep(
        (r) => ({ ...r, settings: { ...r.settings, next_pick_team_id: teamId } }),
        () => saveNextPick(room.id, teamId),
      );
    },

    /** Tells the room the running score and whose ban or pick is next. */
    announceScore: () => command({ type: "announce_score" }),
    settings: () => command({ type: "settings" }),
    lock: (locked: boolean) => command({ type: "lock", locked }),
    timer: (seconds: number) => command({ type: "timer", seconds }),
    abortTimer: () => command({ type: "abort_timer" }),
    start: (seconds: number) => command({ type: "start", seconds }),
    abort: () => command({ type: "abort" }),
    close: () => command({ type: "close" }),
    /** The bot tries the room's channel again: a room made elsewhere, once its maker added the bot. */
    rejoin: () => command({ type: "rejoin" }),

    /** Leaves a finished map out of the score ("Sayma"), or counts it again. */
    toggleIgnored: (key: string) => {
      if (!room) return;
      const ignore = !ignored.has(key);
      keep(
        (r) => ({
          ...r,
          ignored_maps: ignore ? [...r.ignored_maps, key] : r.ignored_maps.filter((k) => k !== key),
        }),
        () => setIgnoredMap(room.id, key, ignore),
      );
    },

    /** Reads osu!'s record of the rooms in place of the chat's maps; the score is saved from it. */
    syncFromOsu: async () => {
      setSyncing(true);
      setError(null);
      try {
        const { osu_synced_at: at } = await syncFromOsu(matchId);
        setRoom((r) => r && { ...r, settings: { ...r.settings, osu_synced_at: at } });
      } catch (e) {
        setError(errorMessage(e));
      } finally {
        setSyncing(false);
      }
    },

    ban: (mapId: string, teamId: string) =>
      run(async () => {
        await setMapBan(matchId, mapId, teamId);
        await reload();
      }),

    unban: (mapId: string) =>
      run(async () => {
        await removeMapBan(matchId, mapId);
        await reload();
      }),

    /** Saves the score as the bots count it, over whatever the site has. */
    saveScore: () =>
      run(async () => {
        await saveRoomScore(matchId);
        await reload();
      }),

    /** Awards a map (or, without one, a point) to a team by hand; it counts towards the score. */
    addManualResult: (teamId: string, mapId: string | null, note: string) =>
      run(async () => {
        await addManualResult(matchId, teamId, mapId, note);
        await reload();
      }),

    removeManualResult: (resultId: string) =>
      run(async () => {
        await removeManualResult(matchId, resultId);
        await reload();
      }),

    /**
     * Concludes the match; a bracket match with the score as the bots count it, saved first if the
     * site doesn't have it, so the winner moves on with it.
     */
    conclude: () =>
      run(async () => {
        if (room?.kind === "bracket" && live?.tally && !scoreSaved) await saveRoomScore(matchId);
        await concludeMatch(matchId);
        await reload();
      }),

    /** Concludes the match by forfeit: `slot` is the team that forfeited; the other one moves on. */
    forfeit: (slot: 1 | 2) =>
      run(async () => {
        await forfeitMatch(matchId, slot);
        await reload();
      }),
  };

  return {
    room,
    live,
    roomChats,
    loadError,
    error,
    busy,
    canRun,
    channel,
    access,
    botName,
    botReady,
    canControl,
    controlsReady,
    autoLock,
    lobby,
    currentEntry,
    mapStartedAt: lobby.mapStartedAt,
    teams,
    draft,
    turn,
    roster,
    findRoster,
    staff,
    options,
    ignored,
    finished,
    source,
    osuSyncedAt: settings?.osu_synced_at ?? null,
    syncing,
    tally,
    scoreSaved,
    bestOf,
    standing,
    streamer,
    streamerIsRef,
    autoEnabled,
    autoref,
    autorefLog,
    actions,
  };
}

export type RefereeRoomState = ReturnType<typeof useRefereeRoom>;

/** Why the controls that referee the match wait: the auto referee runs it. */
export const AUTO_LOCK = "Otomatik hakem maçı yönetiyor";

/**
 * Why a control that has the room's bot referee the match can't be used right now: the bot can't
 * (botBlocker), or the auto referee runs the match. Null when it can.
 */
export function controlBlocker(
  referee: Pick<RefereeRoomState, "room" | "canRun" | "access" | "botName" | "autoLock">,
): string | null {
  return botBlocker(referee) ?? referee.autoLock;
}

/** Why the room's bot can't take a command right now, or null when it can. */
export function botBlocker(
  referee: Pick<RefereeRoomState, "room" | "canRun" | "access" | "botName">,
): string | null {
  const { room, canRun, access, botName } = referee;
  const bot = botName ?? "Bot";
  if (!room) return "Yükleniyor";
  if (!canRun) return "Bu maçın hakemi değilsin";
  if (room.concluded_at) return "Maç bitti";
  switch (access) {
    case null:
      return "Önce oda aç";
    case "in":
      return null;
    case "joining":
      return `${bot} odaya giriyor…`;
    case "refused":
      return `${bot} odaya giremiyor: odayı açan kişi !mp addref ${bot} yazmalı`;
    case "offline":
      return `${bot} Bancho'ya bağlı değil: yeni oda aç`;
    case "no_bot":
      return "Bu odayı bir bot yönetmiyor: yeni oda aç";
    case "closed":
      return "Oda kapandı";
  }
}
