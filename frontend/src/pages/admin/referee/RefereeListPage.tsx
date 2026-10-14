import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { fetchBracket } from "../../../api/bracket";
import { errorMessage } from "../../../api/client";
import { fetchQualifiers } from "../../../api/matches";
import type { MatchReferee } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { matchCode } from "../../../lib/bracket";
import { formatShortMatchTime } from "../../../lib/format";
import "./RefereeListPage.css";

interface MatchItem {
  id: string;
  label: string;
  teams: string;
  startsAt: string | null;
  referee: MatchReferee | null;
  concluded: boolean;
}

async function loadMatches(): Promise<MatchItem[]> {
  const [qualifiers, bracket] = await Promise.all([fetchQualifiers(), fetchBracket()]);
  const teamName = new Map(bracket.teams.map((t) => [t.id, t.name]));
  const items: MatchItem[] = [
    ...qualifiers.lobbies.map((l, i) => ({
      id: l.id,
      label: `Lobi ${i + 1}`,
      teams: `${l.teams.length}/${qualifiers.capacity} takım`,
      startsAt: l.starts_at,
      referee: l.referee,
      concluded: l.concluded_at !== null,
    })),
    ...bracket.matches.map((m) => ({
      id: m.id,
      label: matchCode(m),
      teams: [m.team1_id, m.team2_id].map((id) => (id && teamName.get(id)) || "?").join(" vs "),
      startsAt: m.starts_at,
      referee: m.referee,
      concluded: m.concluded_at !== null,
    })),
  ];
  // Unfinished first, then by time; unscheduled matches last.
  return items.sort(
    (a, b) =>
      Number(a.concluded) - Number(b.concluded) ||
      (a.startsAt ?? "￿").localeCompare(b.startsAt ?? "￿") ||
      a.label.localeCompare(b.label),
  );
}

/** /admin/referee — every match, the referee's own first, each opening its referee room. */
export function RefereeListPage() {
  const user = useSignedInUser();
  const [items, setItems] = useState<MatchItem[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    loadMatches()
      .then(setItems)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  if (error) return <ErrorAlert message={error} />;
  if (!items) return <p className="status">Yükleniyor…</p>;

  const mine = items.filter((m) => m.referee?.id === user.id);
  const others = items.filter((m) => m.referee?.id !== user.id);

  return (
    <div className="referee-list-page">
      <MatchList title="Hakemi olduğun maçlar" empty="Henüz hakemliğini üstlendiğin maç yok." items={mine} />
      <MatchList title="Diğer maçlar" empty="Başka maç yok." items={others} />
    </div>
  );
}

function MatchList({ title, empty, items }: { title: string; empty: string; items: MatchItem[] }) {
  return (
    <section className="referee-list">
      <h2 className="referee-list-title">{title}</h2>
      {items.length === 0 ? (
        <p className="muted">{empty}</p>
      ) : (
        <ul>
          {items.map((m) => (
            <li key={m.id}>
              <Link to={`/admin/referee/${m.id}`} className={`referee-list-item ${m.concluded ? "is-concluded" : ""}`}>
                <span className="referee-list-code">{m.label}</span>
                <span className="referee-list-teams">{m.teams}</span>
                <span className="referee-list-time">{m.startsAt ? formatShortMatchTime(m.startsAt) : "Saat yok"}</span>
                <span className="referee-list-ref">{m.referee?.username ?? <span className="muted">Hakem yok</span>}</span>
                {m.concluded && <span className="badge badge-success">Bitti</span>}
              </Link>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
