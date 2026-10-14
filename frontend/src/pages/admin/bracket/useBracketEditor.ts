import { useCallback, useEffect, useRef, useState } from "react";
import {
  createBracketMatch,
  deleteBracketMatch,
  fetchBracket,
  fetchPublicBracket,
  moveBracketMatch,
  setBracketScore,
  setStreamer,
  updateBracketMatch,
  type BracketMatchUpdate,
} from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import {
  addMatchRoom,
  concludeMatch,
  removeMatchRoom,
  reopenMatch,
  setMatchNotes,
  setReferee,
} from "../../../api/matches";
import type { AdminTeamPlayer, BracketMatch, BracketSide, Player, Team } from "../../../api/types";
import { overlapsAny } from "../../../lib/bracket";

/** Referees and up get the staff view with notes; streamers get the public one. */
export type EditorMatch = BracketMatch & { referee_notes?: string };
export type EditorTeam = Team<Player | AdminTeamPlayer>;

export interface EditorData {
  matches: EditorMatch[];
  teams: EditorTeam[];
  /** The viewer's own team; only streamers can have one. */
  myTeamId: string | null;
}

// Moves wait this long for the next one before saving, so nudging a card across the
// board with the arrow keys sends one request rather than one per step.
const MOVE_SAVE_DELAY_MS = 400;

/** Owns the bracket admin page's data and every API call it makes. */
export function useBracketEditor(staffView: boolean) {
  const [data, setData] = useState<EditorData | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // Moves waiting to be saved, by match id.
  const pendingMoves = useRef(new Map<string, { timer: number; save: () => Promise<void> }>());

  const reload = useCallback(async () => {
    if (staffView) {
      const d = await fetchBracket();
      setData({ matches: d.matches, teams: d.teams, myTeamId: null });
    } else {
      const d = await fetchPublicBracket();
      setData({ matches: d.matches, teams: d.teams, myTeamId: d.my_team_id });
    }
  }, [staffView]);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  // Saves waiting moves now, so a reload can't put their cards back.
  const flushMoves = useCallback(async () => {
    const saves = [...pendingMoves.current.values()];
    pendingMoves.current.clear();
    for (const { timer } of saves) window.clearTimeout(timer);
    await Promise.all(saves.map(({ save }) => save()));
  }, []);

  // Structural changes: busy while they run, then the whole bracket reloads, since
  // concluding or reopening a match also changes the matches it leads into.
  const run = useCallback(
    async (action: () => Promise<void>) => {
      setBusy(true);
      setError(null);
      try {
        await flushMoves();
        await action();
      } catch (e) {
        setError(errorMessage(e));
      }
      try {
        await reload();
      } catch (e) {
        setError(errorMessage(e));
      } finally {
        setBusy(false);
      }
    },
    [reload, flushMoves],
  );

  const patchMatches = (change: (m: EditorMatch) => EditorMatch) =>
    setData((d) => d && { ...d, matches: d.matches.map(change) });

  const background = (request: Promise<unknown>) =>
    request.then(
      () => {},
      (e) => {
        setError(errorMessage(e));
        reload().catch(() => {});
      },
    );

  /** Moves a match on the board right away and saves shortly after. Cards can't overlap. */
  function move(id: string, x: number, y: number) {
    if (!data || overlapsAny(data.matches, { x, y }, id)) {
      setError("Orada başka bir maç var.");
      return;
    }
    patchMatches((m) => (m.id === id ? { ...m, pos_x: x, pos_y: y } : m));
    window.clearTimeout(pendingMoves.current.get(id)?.timer);
    const save = () => background(moveBracketMatch(id, x, y));
    const timer = window.setTimeout(() => {
      pendingMoves.current.delete(id);
      save();
    }, MOVE_SAVE_DELAY_MS);
    pendingMoves.current.set(id, { timer, save });
  }

  // Notes save in the background so typing never waits on the server.
  function saveNotes(id: string, notes: string) {
    patchMatches((m) => (m.id === id ? { ...m, referee_notes: notes } : m));
    background(setMatchNotes(id, notes));
  }

  /** Adds a match and resolves to its id, or null if it failed. */
  async function add(side: BracketSide, x: number, y: number): Promise<string | null> {
    let id: string | null = null;
    await run(async () => {
      id = (await createBracketMatch(side, x, y)).id;
    });
    return id;
  }

  return {
    data,
    error,
    setError,
    busy,
    move,
    saveNotes,
    add,
    update: (id: string, update: BracketMatchUpdate) => run(() => updateBracketMatch(id, update)),
    remove: (id: string) => run(() => deleteBracketMatch(id)),
    setScore: (id: string, team1: number | null, team2: number | null) =>
      run(() => setBracketScore(id, team1, team2)),
    addRoom: (id: string, link: string) => run(() => addMatchRoom(id, link)),
    removeRoom: (id: string, mpId: number) => run(() => removeMatchRoom(id, mpId)),
    setReferee: (id: string, userId: string | null) => run(() => setReferee(id, userId)),
    setStreamer: (id: string, userId: string | null) => run(() => setStreamer(id, userId)),
    conclude: (id: string) => run(() => concludeMatch(id)),
    reopen: (id: string) => run(() => reopenMatch(id)),
  };
}

export type BracketEditor = ReturnType<typeof useBracketEditor>;
