// A stage's pick and ban order (Stage.draft_order), as hosts write it: "p1-p2-b2-b1-p1-*". `p` is
// a map picked and played, `b` a ban; 1 is the team that marks the first ban or pick on the
// referee page (the roll decides it), 2 the other; a closing `*` has the teams take turns picking
// from there. The server's rules are backend/src/draft.rs; this mirrors them to show the draft.

export type StepKind = "ban" | "pick";

export interface OrderStep {
  kind: StepKind;
  team: 1 | 2;
}

export interface DraftOrder {
  steps: OrderStep[];
  /** A closing `*`: picks take turns after the last step. */
  alternate: boolean;
}

/** What a stage plays without an order of its own: two bans, then picks in turn. */
export const DEFAULT_DRAFT_ORDER = "b1-b2-p2-*";
/** The order the tournament plays by, offered as the example. */
export const EXAMPLE_DRAFT_ORDER = "p1-p2-b2-b1-p1-*";
const MAX_STEPS = 40;

/** Reads an order, ignoring case and spaces; the error says what's wrong, in Turkish. */
export function parseDraftOrder(text: string): { order: DraftOrder } | { error: string } {
  const tokens = text.replace(/\s+/g, "").toLowerCase().split("-");
  const alternate = tokens[tokens.length - 1] === "*";
  if (alternate) tokens.pop();
  if (tokens.length === 0 || (tokens.length === 1 && tokens[0] === "")) {
    return { error: "En az bir pick ya da ban gerekli." };
  }
  if (tokens.length > MAX_STEPS) return { error: `En fazla ${MAX_STEPS} adım olabilir.` };
  const steps: OrderStep[] = [];
  for (const token of tokens) {
    const match = /^([pb])([12])$/.exec(token);
    if (!match) {
      return {
        error:
          token === "*"
            ? "* yalnızca sonda olabilir."
            : `"${token || "-"}" tanınmadı: her adım p1, p2, b1 ya da b2 olmalı.`,
      };
    }
    steps.push({ kind: match[1] === "p" ? "pick" : "ban", team: match[2] === "1" ? 1 : 2 });
  }
  return { order: { steps, alternate } };
}

/** How it's stored: lower case, no spaces. */
export function formatDraftOrder(order: DraftOrder): string {
  const parts = order.steps.map((s) => `${s.kind === "pick" ? "p" : "b"}${s.team}`);
  return [...parts, ...(order.alternate ? ["*"] : [])].join("-");
}

/** A stage's order, or the default when it has none or it doesn't read. */
export function stageDraftOrder(stored: string | null | undefined): DraftOrder {
  const parsed = parseDraftOrder(stored || DEFAULT_DRAFT_ORDER);
  return "order" in parsed
    ? parsed.order
    : (parseDraftOrder(DEFAULT_DRAFT_ORDER) as { order: DraftOrder }).order;
}

interface Mark {
  map_id: string;
  team_id: string;
  created_at: string;
}

/** One step of a match's draft: a ban or pick made, the one whose turn it is, or one to come. */
export interface DraftStep {
  kind: StepKind;
  /** The order's 1 or 2; null past its listed steps, where picks take turns. */
  number: 1 | 2 | null;
  /** Who made it, or whose it is by the order (known once the first ban or pick says who 1 is). */
  teamId: string | null;
  /** The pool map, once made. */
  mapId: string | null;
  state: "done" | "next" | "later";
  /** Made as a ban where the order has a pick, or the other way round. */
  offOrder: boolean;
}

export interface Draft {
  steps: DraftStep[];
  /** The next ban or pick, while the draft is open and the order hasn't run out. */
  next: { kind: StepKind; teamId: string | null } | null;
  /** Picks go on taking turns after the steps listed. */
  alternate: boolean;
}

/**
 * A match's draft: its bans and picks in the order they were marked, then what the order has
 * still to come. `chosen` is the team the referee said goes next, which stands in for the order
 * until the next mark. Closed (the tiebreaker, a decided match), only what was made is listed.
 */
export function draftOf(
  order: DraftOrder,
  bans: Mark[],
  picks: Mark[],
  teams: [string, string],
  chosen: string | null,
  open: boolean,
): Draft {
  const marks = [
    ...bans.map((m) => ({ ...m, kind: "ban" as const })),
    ...picks.map((m) => ({ ...m, kind: "pick" as const })),
  ].sort((a, b) => Date.parse(a.created_at) - Date.parse(b.created_at));
  const other = (id: string) => (id === teams[0] ? teams[1] : teams[0]);
  // The first mark was made by the first step's team.
  const first = marks[0]?.team_id ?? null;
  const one = first === null ? null : order.steps[0].team === 1 ? first : other(first);
  const teamOf = (n: 1 | 2) => (one === null ? null : n === 1 ? one : other(one));

  const steps: DraftStep[] = marks.map((m, i) => {
    const step = order.steps[i];
    return {
      kind: m.kind,
      number: step?.team ?? null,
      teamId: m.team_id,
      mapId: m.map_id,
      state: "done",
      offOrder: step !== undefined && step.kind !== m.kind,
    };
  });

  let next: Draft["next"] = null;
  if (open) {
    order.steps.slice(marks.length).forEach((step, i) => {
      steps.push({
        kind: step.kind,
        number: step.team,
        teamId: teamOf(step.team),
        mapId: null,
        state: i === 0 ? "next" : "later",
        offOrder: false,
      });
    });
    if (marks.length >= order.steps.length && order.alternate) {
      const last = marks[marks.length - 1]?.team_id;
      steps.push({
        kind: "pick",
        number: null,
        teamId: last ? other(last) : null,
        mapId: null,
        state: "next",
        offOrder: false,
      });
    }
    const upcoming = steps.find((s) => s.state === "next");
    if (upcoming) {
      const override = chosen !== null && teams.includes(chosen) ? chosen : null;
      upcoming.teamId = override ?? upcoming.teamId;
      next = { kind: upcoming.kind, teamId: upcoming.teamId };
    }
  }
  return { steps, next, alternate: order.alternate };
}

/** "Ban" / "Pick", as the referee page says them. */
export function stepLabel(kind: StepKind): string {
  return kind === "ban" ? "Ban" : "Pick";
}
