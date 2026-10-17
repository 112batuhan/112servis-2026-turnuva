import { Icon } from "../../../components/Icon";
import { scoreMessage } from "./pool";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./MatchScore.css";

/**
 * Tells the room the running score and whose pick is next, in the match column's console. The
 * pick follows the rules (see nextPicker) until the referee chooses the other team.
 */
export function AnnounceScore({ referee }: { referee: RefereeRoomState }) {
  const { room, teams, tally, nextPick, inChannel, canRun, actions, race } = referee;
  if (!room || !tally || teams.length !== 2) return null;
  const message = scoreMessage(teams, [tally.team1, tally.team2], nextPick);
  // Nobody picks the tiebreaker, nor after a team has won.
  const drafting = race?.kind !== "tiebreaker" && race?.kind !== "won";
  return (
    <div className="announce" role="group" aria-label="Skoru odaya yaz">
      {drafting && (
        <>
          <span className="announce-label" id="announce-next">
            Sıradaki pick
          </span>
          <span className="announce-teams" role="group" aria-labelledby="announce-next">
            {teams.map((team) => (
              <button
                key={team.id}
                type="button"
                className={`announce-team is-team-${team.color}`}
                aria-pressed={nextPick?.id === team.id}
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
        disabled={!inChannel}
        title={inChannel ? `Odaya yazılacak: ${message}` : "Önce odanın kanalına katıl"}
        aria-description={message}
        onClick={actions.announceScore}
      >
        <Icon name="send" size={15} />
        Skoru odaya yaz
      </button>
    </div>
  );
}
