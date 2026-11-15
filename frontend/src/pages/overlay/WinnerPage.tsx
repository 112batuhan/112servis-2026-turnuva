import { useMemo, type CSSProperties } from "react";
import { API_URL } from "../../api/client";
import type { BracketMatch, PublicBracket, Team } from "../../api/types";
import { raceOf } from "../../lib/bestOf";
import { matchCode } from "../../lib/bracket";
import {
  BannerChip,
  MapPill,
  mapMarks,
  Masthead,
  OverlayStage,
  SIDE_CLASS,
  TrophyGlow,
  type MapMark,
  type Side,
} from "./parts";
import type { BroadcastMatch } from "./useBroadcastMatch";
import "./WinnerPage.css";

// The winner screen after the match: the winning team's banner, name and players large, the final
// score, the maps in the order they were played with who took each, and where both teams go next
// (against whom, once that's decided), stacked in the bottom left so the bottom right stays free
// for the streamer's own overlay. It's drawn from the match as the overlay shows it, so a result
// the tournament clients haven't reached yet isn't revealed here either.

export function WinnerPage({
  view,
  bracket,
}: {
  view: BroadcastMatch | null;
  bracket: PublicBracket | null;
}) {
  const match = view?.page.match ?? null;
  const tally = view?.page.tally ?? null;
  const team = (side: Side): Team | null => {
    const id = side === 1 ? match?.team1_id : match?.team2_id;
    return match?.teams.find((t) => t.id === id) ?? null;
  };

  // The winner: a forfeit's other team, the saved score once it's concluded, or a race already won.
  const winner = useMemo<Side | null>(() => {
    if (!match || !tally) return null;
    if (match.forfeit_slot) return match.forfeit_slot === 1 ? 2 : 1;
    if (match.concluded_at)
      return tally.team1 > tally.team2 ? 1 : tally.team2 > tally.team1 ? 2 : null;
    const bestOf = match.stage?.best_of;
    const race = bestOf ? raceOf(bestOf, tally.team1, tally.team2) : null;
    return race?.kind === "won" ? race.slot : null;
  }, [match, tally]);

  const marks = useMemo(
    () => (match ? mapMarks(match, tally) : new Map<string, MapMark>()),
    [match, tally],
  );
  const node = bracket?.matches.find((m) => m.id === match?.id) ?? null;

  if (!match || !tally || winner === null) {
    return (
      <OverlayStage className="wn">
        <div className="wn-waiting">
          <p className="wn-waiting-teams">
            <span className="is-side-1">{team(1)?.name ?? ""}</span>
            <span className="wn-waiting-score">
              {tally ? `${tally.team1} – ${tally.team2}` : "vs"}
            </span>
            <span className="is-side-2">{team(2)?.name ?? ""}</span>
          </p>
          <p className="wn-waiting-note">Maç bitince kazanan burada</p>
        </div>
        <TrophyGlow className="wn-trophy" />
      </OverlayStage>
    );
  }

  const loser: Side = winner === 1 ? 2 : 1;
  const won = team(winner);
  const lost = team(loser);
  const figure = (side: Side) =>
    match.forfeit_slot
      ? side === winner
        ? "W"
        : "FF"
      : String(side === 1 ? tally.team1 : tally.team2);

  return (
    <OverlayStage className={`wn ${SIDE_CLASS[winner]}`}>
      <Masthead title="Kazanan" />
      <div className="wn-banner">
        {won?.banner_url ? (
          <BannerImage team={won} />
        ) : (
          <span className="wn-banner-initial">{initial(won)}</span>
        )}
      </div>
      <h2 className="wn-name">{won?.name}</h2>
      <ul className="wn-players" aria-label="Oyuncular">
        {won?.players.map((p) => (
          <li key={p.id}>
            {p.avatar_url ? (
              <img src={p.avatar_url} alt="" />
            ) : (
              <span className="wn-avatar-blank" />
            )}
            {p.username}
          </li>
        ))}
      </ul>
      <p className="wn-score">
        <span className={`wn-figure ${SIDE_CLASS[winner]}`}>{figure(winner)}</span>
        <span className="wn-dash">–</span>
        <span className="wn-figure is-loser">{figure(loser)}</span>
        <span className="wn-against">
          <span className={SIDE_CLASS[loser]}>{lost?.name}</span> karşısında
        </span>
      </p>
      {tally.rows.length > 0 && (
        <ol className="wn-maps" aria-label="Oynanan mapler">
          {tally.rows
            .filter((r) => r.counted)
            .map((r, i) => {
              const mark = marks.get(r.entry.map.id);
              return (
                <li key={r.key} style={{ "--wn-i": i } as CSSProperties}>
                  <MapPill
                    entry={r.entry}
                    mark={mark}
                    tag={r.entry.label}
                    current={false}
                    title={false}
                  />
                </li>
              );
            })}
        </ol>
      )}
      {node && (
        <div className="wn-next">
          <NextStep
            winner
            side={winner}
            team={won}
            next={target(bracket, node, "winner")}
            teams={bracket?.teams ?? []}
          />
          <NextStep
            winner={false}
            side={loser}
            team={lost}
            next={target(bracket, node, "loser")}
            teams={bracket?.teams ?? []}
          />
        </div>
      )}
      <TrophyGlow className="wn-trophy" />
    </OverlayStage>
  );
}

function BannerImage({ team }: { team: Team }) {
  return <img src={`${API_URL}${team.banner_url}`} alt="" />;
}

const initial = (team: Team | null) => team?.name.trim().charAt(0).toLocaleUpperCase("tr-TR") ?? "";

/** Where a team goes from this match: the match it moves into and its slot there, or null when it has none. */
function target(
  bracket: PublicBracket | null,
  node: BracketMatch,
  outcome: "winner" | "loser",
): { match: BracketMatch; slot: 1 | 2 } | null {
  const link = outcome === "winner" ? node.winner_to : node.loser_to;
  const match = link ? bracket?.matches.find((m) => m.id === link.match_id) : undefined;
  return link && match ? { match, slot: link.slot } : null;
}

/**
 * A team's next step: its name, then the match it moves into as a chip, naming the team it meets
 * there once that's decided (the stage until then), or, with no match to go to, the end of its
 * road: the winner of a match leading nowhere is the champion, the loser is out.
 */
function NextStep({
  winner,
  side,
  team,
  next,
  teams,
}: {
  winner: boolean;
  side: Side;
  team: Team | null;
  next: { match: BracketMatch; slot: 1 | 2 } | null;
  teams: Team[];
}) {
  const opponentId = next ? (next.slot === 1 ? next.match.team2_id : next.match.team1_id) : null;
  const opponent = opponentId ? teams.find((t) => t.id === opponentId) : undefined;
  return (
    <p className={`wn-next-row ${winner ? "is-winner" : "is-loser"} ${SIDE_CLASS[side]}`}>
      <span className="wn-next-team">{team?.name}</span>
      {next ? (
        <span className="wn-next-where">
          <span className={`wn-next-code is-${next.match.side}`}>{matchCode(next.match)}</span>
          {opponent ? (
            <>
              <span className="wn-next-vs">vs</span>
              <BannerChip team={opponent} className="wn-next-banner" />
              {opponent.name}
            </>
          ) : (
            next.match.stage?.name
          )}
        </span>
      ) : (
        <span className="wn-next-where">{winner ? "Şampiyon" : "Elendi"}</span>
      )}
    </p>
  );
}
