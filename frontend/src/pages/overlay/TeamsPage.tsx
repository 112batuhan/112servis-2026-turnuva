import type { BracketMatch, PublicBracket, PublicMatch, Team } from "../../api/types";
import { bestOfLabel } from "../../lib/bestOf";
import { matchCode, scoreHalves, winningSlot } from "../../lib/bracket";
import { formatRank, formatShortMatchTime } from "../../lib/format";
import { roomLabel } from "../admin/referee/liveRoom";
import { BannerChip, Crystal, Masthead, OverlayStage, SIDE_CLASS, TrophyGlow, type Side } from "./parts";
import type { BroadcastMatch } from "./useBroadcastMatch";
import { useTeamHistory } from "./useOverlayData";
import "./TeamsPage.css";

// The teams before the match: each side's banner, name, qualifier seed and record, its players with
// their ranks, and the bracket matches it has played so far, facing each other across the "vs".

// The most earlier matches a side lists; the rest still count in its record.
const HISTORY_ROWS = 4;

export function TeamsPage({
  view,
  bracket,
  seeds,
}: {
  view: BroadcastMatch | null;
  bracket: PublicBracket | null;
  seeds: ReadonlyMap<string, number>;
}) {
  const match = view?.page.match ?? null;
  const line = match ? matchLine(match) : null;
  return (
    <OverlayStage className="tm">
      <Masthead title={match?.stage?.name ?? "Takımlar"} line={line} />
      <TeamSide side={1} match={match} bracket={bracket} seeds={seeds} />
      <span className="tm-vs" aria-hidden="true">
        vs
      </span>
      <TeamSide side={2} match={match} bracket={bracket} seeds={seeds} />
      <TrophyGlow className="tm-trophy" />
    </OverlayStage>
  );
}

/** "Bo13 · W33 · 8 Kas Paz 21:00". */
function matchLine(match: PublicMatch): string {
  return [
    match.stage?.best_of ? bestOfLabel(match.stage.best_of) : null,
    roomLabel(match),
    match.starts_at ? formatShortMatchTime(match.starts_at) : null,
  ]
    .filter(Boolean)
    .join(" · ");
}

function TeamSide({
  side,
  match,
  bracket,
  seeds,
}: {
  side: Side;
  match: PublicMatch | null;
  bracket: PublicBracket | null;
  seeds: ReadonlyMap<string, number>;
}) {
  const teamId = (side === 1 ? match?.team1_id : match?.team2_id) ?? null;
  const team = match?.teams.find((t) => t.id === teamId) ?? null;
  const history = useTeamHistory(bracket, teamId, match?.id ?? null);
  const wins = history.filter((m) => sideOf(m, teamId) === winningSlot(m)).length;
  const seed = teamId ? seeds.get(teamId) : undefined;

  return (
    <section className={`tm-side ${SIDE_CLASS[side]}`} aria-label={team?.name ?? "Takım"}>
      <BannerChip team={team} className="tm-banner" />
      <h2 className="tm-name">{team?.name ?? "Bekleniyor"}</h2>
      <p className="tm-facts">
        {seed !== undefined && (
          <span className="tm-fact">
            Elemeler <b>#{seed}</b>
          </span>
        )}
        {history.length > 0 && (
          <span className="tm-fact">
            Bracket{" "}
            <b>
              {wins}–{history.length - wins}
            </b>
          </span>
        )}
      </p>
      <ul className="tm-players" aria-label="Oyuncular">
        {team?.players.map((p) => (
          <li key={p.id}>
            {p.avatar_url ? <img src={p.avatar_url} alt="" /> : <span className="tm-avatar-blank" />}
            <span className="tm-player-name">{p.username}</span>
            <span className="tm-rank">{formatRank(p.global_rank)}</span>
          </li>
        ))}
      </ul>
      {history.length > 0 && (
        <>
          <h3 className="tm-history-head">Turnuvadaki maçları</h3>
          <ol className="tm-history">
            {history.slice(0, HISTORY_ROWS).map((m) => (
              <HistoryRow key={m.id} match={m} teamId={teamId} teams={bracket?.teams ?? []} />
            ))}
          </ol>
        </>
      )}
    </section>
  );
}

/** 1 or 2: the slot the team played a match in. */
function sideOf(m: BracketMatch, teamId: string | null): Side | null {
  return m.team1_id === teamId ? 1 : m.team2_id === teamId ? 2 : null;
}

/**
 * An earlier match from the team's side: a filled crystal for a win, a hollow one for a loss, the
 * code, the opponent and the score with the team's own figure first.
 */
function HistoryRow({ match, teamId, teams }: { match: BracketMatch; teamId: string | null; teams: Team[] }) {
  const own = sideOf(match, teamId);
  const won = own !== null && winningSlot(match) === own;
  const opponent = teams.find((t) => t.id === (own === 1 ? match.team2_id : match.team1_id));
  const halves = scoreHalves(match);
  const score = halves && own ? (own === 1 ? `${halves[0]}–${halves[1]}` : `${halves[1]}–${halves[0]}`) : "";
  return (
    <li className={won ? "is-won" : "is-lost"}>
      <Crystal kind={won ? "won" : "open"} className="tm-result" />
      <span className={`tm-code is-${match.side}`}>{matchCode(match)}</span>
      <span className="tm-opponent">{opponent?.name ?? "?"}</span>
      <span className="tm-score">{score}</span>
    </li>
  );
}
