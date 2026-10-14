import { useMemo, useState } from "react";
import type { BracketMatch, Team } from "../../api/types";
import { matchCode } from "../../lib/bracket";
import "./BracketSearch.css";

/** Everything the search matches against, lowercased: the code, both teams and their players, and the referee. */
function searchText(match: BracketMatch, teams: Map<string, Team>): string {
  const parts = [matchCode(match), match.referee?.username ?? ""];
  for (const id of [match.team1_id, match.team2_id]) {
    const team = id ? teams.get(id) : undefined;
    if (team) parts.push(team.name, ...team.players.map((p) => p.username));
  }
  return parts.join("\n").toLowerCase();
}

/** The search shared by the bracket's views; `hits` is the ids of the matching matches, or null while it's empty. */
export function useBracketSearch(matches: BracketMatch[], teams: Map<string, Team>) {
  const [query, setQuery] = useState("");
  const hits = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return null;
    return new Set(matches.filter((m) => searchText(m, teams).includes(needle)).map((m) => m.id));
  }, [matches, teams, query]);
  return { query, setQuery, hits };
}

interface Props {
  query: string;
  onChange: (query: string) => void;
  hits: Set<string> | null;
}

export function BracketSearch({ query, onChange, hits }: Props) {
  return (
    <span className="bracket-search">
      <input
        className="input input-sm"
        type="search"
        placeholder="Maç, takım, oyuncu veya hakem ara"
        aria-label="Maç ara"
        value={query}
        onChange={(e) => onChange(e.target.value)}
      />
      {hits && (
        <span className="bracket-search-count" role="status">
          {hits.size} eşleşme
        </span>
      )}
    </span>
  );
}
