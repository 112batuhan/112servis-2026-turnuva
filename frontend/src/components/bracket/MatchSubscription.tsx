import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../../api/client";
import { fetchSubscriptions, subscribe, unsubscribe } from "../../api/discord";
import type { BracketMatch, Team } from "../../api/types";
import { hasMatchPage, isLive, matchCode } from "../../lib/bracket";
import { Icon } from "../Icon";
import { IconButton } from "../IconButton";
import { MatchPageLink } from "../MatchPageLink";
import "./MatchSubscription.css";

export interface SubscriptionState {
  /** null while loading, when signed out, or when the Discord bot is off. */
  ids: Set<string> | null;
  linked: boolean;
  toggle: (matchId: string) => void;
  error: string | null;
  clearError: () => void;
}

/**
 * The bracket matches the signed-in user wants to be pinged for on Discord shortly before they
 * start. Toggling updates at once and goes back if saving fails.
 */
export function useSubscriptions(signedIn: boolean): SubscriptionState {
  const [ids, setIds] = useState<Set<string> | null>(null);
  const [linked, setLinked] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!signedIn) {
      setIds(null);
      return;
    }
    fetchSubscriptions()
      .then((s) => {
        setLinked(s.discord_linked);
        setIds(s.enabled ? new Set(s.match_ids) : null);
      })
      .catch(() => setIds(null));
  }, [signedIn]);

  const toggle = useCallback(
    (matchId: string) => {
      if (!ids) return;
      const on = !ids.has(matchId);
      const flip = (set: Set<string>, add: boolean) => {
        const next = new Set(set);
        if (add) next.add(matchId);
        else next.delete(matchId);
        return next;
      };
      setIds((set) => set && flip(set, on));
      setError(null);
      (on ? subscribe(matchId) : unsubscribe(matchId)).catch((e) => {
        setIds((set) => set && flip(set, !on));
        setError(errorMessage(e));
      });
    },
    [ids],
  );

  return { ids, linked, toggle, error, clearError: () => setError(null) };
}

const canSubscribe = (m: BracketMatch) => m.concluded_at === null;

/** A bell that subscribes to a match, for the list's rows. */
export function SubscribeButton({ match, subs }: { match: BracketMatch; subs: SubscriptionState }) {
  if (!subs.ids || !canSubscribe(match)) return null;
  if (!subs.linked) {
    return (
      <IconButton
        icon="bell"
        label="Maç başlarken haber almak için Discord hesabını bağla"
        disabled
      />
    );
  }
  const on = subs.ids.has(match.id);
  return (
    <IconButton
      icon="bell"
      active={on}
      aria-pressed={on}
      label={
        on
          ? "Discord'da haber verilecek, vazgeçmek için tıkla"
          : "Maç başlarken Discord'da etiketle"
      }
      onClick={() => subs.toggle(match.id)}
    />
  );
}

/** The selected match with its subscribe toggle, above the grid and the calendar, for signed-in users. */
export function SelectedMatchBar({
  match,
  teams,
  subs,
}: {
  match: BracketMatch | undefined;
  teams: Map<string, Team>;
  subs: SubscriptionState;
}) {
  if (!subs.ids) return null;
  // Stays in place without a selection, so selecting a match doesn't move the board.
  if (!match || !canSubscribe(match)) {
    return (
      <div className="match-sub-bar">
        <span className="muted">
          {match
            ? "Bu maç bitti."
            : "Bir maç seç: başlamadan önce Discord'da etiketlenmek için abone olabilirsin."}
        </span>
        {match && hasMatchPage(match) && <MatchPageLink matchId={match.id} variant="text" />}
      </div>
    );
  }
  const name = (id: string | null) => (id ? (teams.get(id)?.name ?? "?") : "Belirlenmedi");
  const on = subs.ids.has(match.id);
  return (
    <div className="match-sub-bar">
      <span className="match-sub-title">
        <strong>{matchCode(match)}</strong> {name(match.team1_id)} <span className="muted">vs</span>{" "}
        {name(match.team2_id)}
      </span>
      {hasMatchPage(match) && (
        <MatchPageLink matchId={match.id} live={isLive(match)} variant="text" />
      )}
      {subs.linked ? (
        <button
          type="button"
          className={on ? "btn btn-primary btn-sm" : "btn btn-secondary btn-sm"}
          aria-pressed={on}
          onClick={() => subs.toggle(match.id)}
        >
          {on ? (
            <>
              <Icon name="check" size={14} />
              Discord'da haber verilecek
            </>
          ) : (
            "Başlarken Discord'da etiketle"
          )}
        </button>
      ) : (
        <span className="muted">
          Maç başlarken Discord'da haber almak için{" "}
          <Link to="/discord">Discord hesabını bağla</Link>.
        </span>
      )}
    </div>
  );
}
