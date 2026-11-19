import { useEffect, useMemo, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { useAuth } from "../auth/AuthContext";
import { fetchPublicBracket } from "../api/bracket";
import { errorMessage } from "../api/client";
import type { BracketMatch, PublicBracket } from "../api/types";
import {
  bracketMatchPath,
  compareByTime,
  fixturePath,
  hasMatchPage,
  isOwnMatch,
  roomPagePath,
  incomingSources,
  rowMatchId,
  withPotentials,
  withScore,
} from "../lib/bracket";
import { useLiveBracket } from "../live/LiveContext";
import { isAdminPath } from "../lib/adminTabs";
import { FixtureRow } from "./FixtureRow";
import { Icon } from "./Icon";
import "./TeamMatches.css";

/**
 * A team's next bracket match (a potential match counts, if it's the soonest) and the ones it has played, newest first. Each one links to the
 * match's public page (the bracket's grid while it waits for a team), or from an admin page to the
 * admin bracket with that match selected. Loads the public bracket when it mounts, so it's fresh every time it's opened.
 */
export function TeamMatches({ teamId }: { teamId: string }) {
  const [data, setData] = useState<PublicBracket | null>(null);
  const [error, setError] = useState<string | null>(null);
  const admin = isAdminPath(useLocation().pathname);
  const { user } = useAuth();

  useEffect(() => {
    fetchPublicBracket()
      .then(setData)
      .catch((e) => setError(errorMessage(e)));
  }, []);
  useLiveBracket(
    (e) =>
      setData(
        (d) =>
          d && { ...d, matches: withScore(d.matches, e.match_id, e.team1_score, e.team2_score) },
      ),
    () =>
      fetchPublicBracket()
        .then(setData)
        .catch(() => {}),
  );

  const teams = useMemo(() => new Map(data?.teams.map((t) => [t.id, t])), [data]);
  const sources = useMemo(() => incomingSources(data?.matches ?? []), [data]);

  if (error) return <p className="status">{error}</p>;
  if (!data) return <p className="status">Yükleniyor…</p>;

  const own = withPotentials(data.matches).filter(
    (m) => m.team1_id === teamId || m.team2_id === teamId,
  );
  const next = own.filter((m) => !m.concluded_at).sort(compareByTime)[0];
  const played = own
    .filter((m) => m.concluded_at)
    .sort((a, b) => (b.concluded_at ?? "").localeCompare(a.concluded_at ?? ""));

  const row = (m: BracketMatch) => (
    <FixtureRow
      match={m}
      to={admin ? bracketMatchPath(rowMatchId(m), true) : fixturePath(m)}
      team1={m.team1_id ? teams.get(m.team1_id) : undefined}
      team2={m.team2_id ? teams.get(m.team2_id) : undefined}
      sources={sources.get(m.id)}
      ownTeamId={teamId}
    />
  );

  return (
    <div className="team-matches">
      <section>
        <h3 className="team-matches-heading">Sıradaki maç</h3>
        {next ? (
          <>
            <div className="fixture-list">{row(next)}</div>
            {!admin &&
              next.potential_of === undefined &&
              hasMatchPage(next) &&
              isOwnMatch(user?.team_id, [next.team1_id, next.team2_id], false) && (
                <p className="team-matches-room">
                  <Link
                    to={roomPagePath(next.id)}
                    className={`btn btn-secondary btn-sm match-page-link is-text is-live`}
                  >
                    <Icon name="door" size={14} />
                    Maç odası
                  </Link>
                  <span>Warmup, ban ve pick'lerinizi maç sırasında buradan seçersiniz.</span>
                </p>
              )}
          </>
        ) : (
          <p className="team-matches-empty">Planlanmış maç yok.</p>
        )}
      </section>
      <section>
        <h3 className="team-matches-heading">
          Maç geçmişi {played.length > 0 && <span className="num">{played.length}</span>}
        </h3>
        {played.length === 0 ? (
          <p className="team-matches-empty">Henüz oynanmış maç yok.</p>
        ) : (
          <ul className="fixture-list">
            {played.map((m) => (
              <li key={m.id}>{row(m)}</li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
