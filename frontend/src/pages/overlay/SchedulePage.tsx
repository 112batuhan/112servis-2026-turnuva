import { useMemo } from "react";
import type { BracketMatch, PublicBracket, Team } from "../../api/types";
import { stageLabel } from "../../lib/bestOf";
import { compareByTime, isLive, matchCode, scoreHalves, winningSlot } from "../../lib/bracket";
import { formatShortMatchTime } from "../../lib/format";
import { BannerChip, Masthead, OverlayStage, SIDE_CLASS, TrophyGlow, type Side } from "./parts";
import { currentStageId } from "./useOverlayData";
import "./SchedulePage.css";

// The schedule between matches: the stage being played, its matches still to come on the left (the
// live one first) and its results on the right, each a capsule with both banners, the code over its
// top edge and the time or the score in the middle.

// Six rows fill a column at the designed spacing.
const ROWS = 6;
// One empty list, so no bracket doesn't make a new one every render.
const NO_MATCHES: never[] = [];

export function SchedulePage({ bracket, matchId }: { bracket: PublicBracket | null; matchId: string | null }) {
  const matches = bracket?.matches ?? NO_MATCHES;
  const onStream = matches.find((m) => m.id === matchId) ?? null;
  const stageId = currentStageId(matches, onStream);
  const stage = matches.find((m) => m.stage?.id === stageId)?.stage ?? null;
  const { upcoming, results } = useMemo(() => {
    const inStage = matches.filter((m) => m.stage?.id === stageId && (m.team1_id || m.team2_id));
    return {
      upcoming: inStage
        .filter((m) => !m.concluded_at)
        .sort((a, b) => Number(isLive(b)) - Number(isLive(a)) || compareByTime(a, b))
        .slice(0, ROWS),
      results: inStage
        .filter((m) => m.concluded_at)
        .sort((a, b) => (b.concluded_at ?? "").localeCompare(a.concluded_at ?? ""))
        .slice(0, ROWS),
    };
  }, [matches, stageId]);
  const teams = bracket?.teams ?? [];

  return (
    <OverlayStage className="sc">
      <Masthead title="Maçlar" line={stage ? stageLabel(stage) : null} />
      <section className="sc-col is-upcoming" aria-label="Sıradaki maçlar">
        <h2>Sıradaki maçlar</h2>
        {upcoming.length === 0 ? (
          <p className="sc-empty">Bu aşamada oynanacak maç kalmadı.</p>
        ) : (
          <ol>
            {upcoming.map((m, i) => (
              <Fixture key={m.id} match={m} teams={teams} onStream={m.id === matchId} index={i} />
            ))}
          </ol>
        )}
      </section>
      <section className="sc-col is-results" aria-label="Sonuçlar">
        <h2>Sonuçlar</h2>
        {results.length === 0 ? (
          <p className="sc-empty">Bu aşamada henüz biten maç yok.</p>
        ) : (
          <ol>
            {results.map((m, i) => (
              <Fixture key={m.id} match={m} teams={teams} onStream={m.id === matchId} index={i} />
            ))}
          </ol>
        )}
      </section>
      <TrophyGlow className="sc-trophy" />
    </OverlayStage>
  );
}

/**
 * One match as a capsule: team 1's banner and name on the left, team 2's on the right, and between
 * them "vs", the running score while it's live, or the result (the winner's figure in its colour,
 * the loser's banner and name dimmed). Its code rides the top edge, with the time or "Canlı".
 */
function Fixture({ match, teams, onStream, index }: { match: BracketMatch; teams: Team[]; onStream: boolean; index: number }) {
  const team = (id: string | null) => teams.find((t) => t.id === id) ?? null;
  const live = isLive(match);
  const winner = winningSlot(match);
  const halves = scoreHalves(match);
  const showScore = (live || match.concluded_at) && halves;
  const side = (slot: Side) => {
    const t = team(slot === 1 ? match.team1_id : match.team2_id);
    const state = winner === null ? "" : winner === slot ? "is-winner" : "is-loser";
    return (
      <div className={`sc-team ${SIDE_CLASS[slot]} ${state}`}>
        <BannerChip team={t} className="sc-banner" />
        <span className={`sc-name ${t ? "" : "is-waiting"}`}>{t?.name ?? "Bekleniyor"}</span>
      </div>
    );
  };
  return (
    <li
      className={`sc-fixture ${live ? "is-live" : ""} ${onStream ? "is-on-stream" : ""}`}
      style={{ animationDelay: `${index * 70}ms` }}
    >
      {side(1)}
      <span className="sc-middle">
        {showScore ? (
          <>
            <span className={`sc-figure is-side-1 ${winner === 2 ? "is-loser" : ""}`}>{halves[0]}</span>
            <span className="sc-dash">–</span>
            <span className={`sc-figure is-side-2 ${winner === 1 ? "is-loser" : ""}`}>{halves[1]}</span>
          </>
        ) : (
          <span className="sc-vs">vs</span>
        )}
      </span>
      {side(2)}
      <span className={`sc-tag sc-code is-${match.side}`}>{matchCode(match)}</span>
      <span className={`sc-tag sc-when ${live ? "is-live" : ""}`}>
        {live ? (
          <>
            <span className="sc-live-dot" aria-hidden="true" />
            Canlı
          </>
        ) : match.concluded_at ? (
          match.forfeit_slot ? "Hükmen" : "Bitti"
        ) : match.starts_at ? (
          formatShortMatchTime(match.starts_at)
        ) : (
          "Zaman yok"
        )}
      </span>
    </li>
  );
}
