import { Icon } from "../../../components/Icon";
import { stepLabel } from "../../../lib/draftOrder";
import { scoreMessage } from "../../../lib/matchPool";
import { botBlocker, type RefereeRoomState } from "./useRefereeRoom";
import "./MatchScore.css";

/**
 * Tells the room the running score and whose ban or pick is next, in the match column's console.
 * The turn follows the stage's order (read from the bans and picks) until the referee chooses the
 * other team.
 */
export function AnnounceScore({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, tally, turn, botReady, canRun, actions } = referee;
  if (!room || !tally || teams.length !== 2) return null;
  const message = scoreMessage(teams, [tally.team1, tally.team2], turn);
  return (
    <div className="announce" role="group" aria-label="Skoru odaya yaz">
      {/* Nobody bans or picks at the tiebreaker, after a team has won, or once the order has run out. */}
      {turn && (
        <>
          <span className="announce-label" id="announce-next">
            Sıradaki {stepLabel(turn.kind).toLowerCase()}
          </span>
          <span className="announce-teams" role="group" aria-labelledby="announce-next">
            {teams.map((team) => (
              <button
                key={team.id}
                type="button"
                className={`announce-team is-team-${team.color}`}
                aria-pressed={turn.team?.id === team.id}
                disabled={!canRun || room.concluded_at !== null}
                onClick={() => actions.setNextPick(team.id)}
              >
                {team.name}
              </button>
            ))}
          </span>
        </>
      )}
      <button
        type="button"
        className="btn btn-secondary announce-send"
        disabled={!botReady}
        title={botReady ? `Bot odaya yazacak: ${message}` : (botBlocker(referee) ?? undefined)}
        aria-description={message}
        onClick={actions.announceScore}
      >
        <Icon name="send" size={15} />
        Skoru odaya yaz
      </button>
    </div>
  );
}
