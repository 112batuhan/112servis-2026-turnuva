import { useEffect, useMemo, useState } from "react";
import { useLocation } from "react-router-dom";
import { fetchPublicBracket } from "../api/bracket";
import { errorMessage } from "../api/client";
import type { BracketMatch, PublicBracket } from "../api/types";
import { bracketMatchPath, compareByTime, incomingSources } from "../lib/bracket";
import { isAdminPath } from "../pages/admin/tabs";
import { FixtureRow } from "./FixtureRow";
import "./TeamMatches.css";

/**
 * A team's next bracket match and the ones it has played, newest first. Each one links to the
 * bracket's grid with that match selected: the admin bracket from an admin page, the public one
 * everywhere else. Loads the public bracket when it mounts, so it's fresh every time it's opened.
 */
export function TeamMatches({ teamId }: { teamId: string }) {
  const [data, setData] = useState<PublicBracket | null>(null);
  const [error, setError] = useState<string | null>(null);
  const admin = isAdminPath(useLocation().pathname);

  useEffect(() => {
    fetchPublicBracket()
      .then(setData)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  const teams = useMemo(() => new Map(data?.teams.map((t) => [t.id, t])), [data]);
  const sources = useMemo(() => incomingSources(data?.matches ?? []), [data]);

  if (error) return <p className="status">{error}</p>;
  if (!data) return <p className="status">Yükleniyor…</p>;

  const own = data.matches.filter((m) => m.team1_id === teamId || m.team2_id === teamId);
  const next = own.filter((m) => !m.concluded_at).sort(compareByTime)[0];
  const played = own
    .filter((m) => m.concluded_at)
    .sort((a, b) => (b.concluded_at ?? "").localeCompare(a.concluded_at ?? ""));

  const row = (m: BracketMatch) => (
    <FixtureRow
      match={m}
      to={bracketMatchPath(m.id, admin)}
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
        {next ? <div className="fixture-list">{row(next)}</div> : <p className="team-matches-empty">Planlanmış maç yok.</p>}
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
