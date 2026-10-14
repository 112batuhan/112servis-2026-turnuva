import { useEffect, useMemo, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { fetchPublicBracket } from "../api/bracket";
import { errorMessage } from "../api/client";
import type { BracketMatch, PublicBracket, Team } from "../api/types";
import {
  bracketMatchPath,
  compareByTime,
  incomingSources,
  matchCode,
  sourceLabel,
  winningSlot,
  type Source,
} from "../lib/bracket";
import { formatShortMatchTime } from "../lib/format";
import { isAdminPath } from "../pages/admin/tabs";
import { Countdown } from "./Countdown";
import { TeamBanner } from "./bracket/TeamBanner";
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

  const row = (m: BracketMatch) => {
    const ours = m.team1_id === teamId ? 1 : 2;
    const theirs = ours === 1 ? 2 : 1;
    const opponentId = theirs === 1 ? m.team1_id : m.team2_id;
    return (
      <MatchRow
        key={m.id}
        match={m}
        to={bracketMatchPath(m.id, admin)}
        ours={ours}
        opponent={opponentId ? teams.get(opponentId) : undefined}
        opponentSource={sources.get(m.id)?.[theirs - 1] ?? null}
      />
    );
  };

  return (
    <div className="team-matches">
      <section>
        <h4 className="team-matches-heading">Sıradaki maç</h4>
        {next ? row(next) : <p className="team-matches-empty">Planlanmış maç yok.</p>}
      </section>
      <section>
        <h4 className="team-matches-heading">
          Maç geçmişi {played.length > 0 && <span className="badge">{played.length}</span>}
        </h4>
        {played.length === 0 ? (
          <p className="team-matches-empty">Henüz oynanmış maç yok.</p>
        ) : (
          <ul className="team-matches-list">
            {played.map((m) => (
              <li key={m.id}>{row(m)}</li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}

interface RowProps {
  match: BracketMatch;
  to: string;
  /** Which slot the team is in. */
  ours: 1 | 2;
  opponent: Team | undefined;
  /** What feeds the opponent's slot while it's empty. */
  opponentSource: Source | null;
}

function MatchRow({ match, to, ours, opponent, opponentSource }: RowProps) {
  const winner = winningSlot(match);
  const ourScore = ours === 1 ? match.team1_score : match.team2_score;
  const theirScore = ours === 1 ? match.team2_score : match.team1_score;
  const upcoming = match.starts_at !== null && new Date(match.starts_at).getTime() > Date.now();
  const result = winner === null ? null : winner === ours ? "win" : "loss";

  return (
    <Link to={to} className="team-matches-row" title="Bracket'te göster">
      <span className="team-matches-meta">
        <span className={`team-matches-code is-${match.side}`}>{matchCode(match)}</span>
        {match.stage && <span>{match.stage.name}</span>}
        <span>{match.starts_at ? formatShortMatchTime(match.starts_at) : "Zaman yok"}</span>
        {!match.concluded_at && upcoming && match.starts_at && (
          <Countdown to={match.starts_at} suffix="kaldı" className="team-matches-countdown" />
        )}
      </span>
      <span className="team-matches-opponent">
        <span className="muted">vs</span>
        <TeamBanner team={opponent} />
        {opponent ? (
          <span className="team-matches-name">{opponent.name}</span>
        ) : (
          <span className="team-matches-name muted">
            {opponentSource ? sourceLabel(opponentSource) : "Belirlenmedi"}
          </span>
        )}
      </span>
      {match.concluded_at && (
        <span className="team-matches-result">
          <span className="team-matches-score">
            {ourScore ?? "–"} – {theirScore ?? "–"}
          </span>
          {result && (
            <span className={result === "win" ? "badge badge-success" : "badge badge-danger"}>
              {result === "win" ? "Kazandı" : "Kaybetti"}
            </span>
          )}
        </span>
      )}
    </Link>
  );
}
