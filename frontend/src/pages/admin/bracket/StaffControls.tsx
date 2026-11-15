import type { Player } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole, holdsRole } from "../../../auth/roles";
import { DiscordMention } from "../../../components/DiscordMention";
import { StaffSelect } from "../MatchControls";
import type { BracketEditor, EditorMatch } from "./useBracketEditor";
import "./StaffControls.css";

interface Props {
  match: EditorMatch;
  editor: BracketEditor;
  /** Who a host can pick from. */
  options: Player[];
}

/**
 * A match's referee: hosts pick from everyone holding the referee role, referees take a free match
 * or give up their own (until it's concluded), and everyone else just sees who it is.
 */
export function RefereeControl({ match, editor, options, discord }: Props & { discord?: boolean }) {
  const user = useSignedInUser();
  const concluded = match.concluded_at !== null;
  // Players contact the referee there, so the list shows it next to the name, as a mark to keep the
  // column narrow.
  const mention = discord && match.referee && (
    <DiscordMention player={match.referee} fallback="" compact />
  );

  if (hasRole(user, "host")) {
    return (
      <span className="staff-control">
        <StaffSelect
          label="Hakem"
          emptyLabel="— Hakem yok —"
          current={match.referee}
          options={options}
          disabled={editor.busy}
          onSelect={(id) => editor.setReferee(match.id, id)}
        />
        {mention}
      </span>
    );
  }
  return (
    <span className="staff-control">
      {match.referee ? (
        <strong>{match.referee.username}</strong>
      ) : (
        <span className="muted">Henüz yok</span>
      )}
      {mention}
      {holdsRole(user, "referee") && !concluded && !match.referee && (
        <button
          type="button"
          className="btn btn-primary btn-sm"
          disabled={editor.busy}
          onClick={() => editor.setReferee(match.id, user.id)}
        >
          Hakemliği üstlen
        </button>
      )}
      {!concluded && match.referee?.id === user.id && (
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={editor.busy}
          onClick={() => editor.setReferee(match.id, null)}
        >
          Bırak
        </button>
      )}
    </span>
  );
}

/**
 * A match's streamer: hosts pick from everyone holding the streamer role, streamers take a free
 * match or give up their own (until it's concluded), except a match their own team plays in, and
 * everyone else just sees who it is.
 */
export function StreamerControl({
  match,
  editor,
  options,
  myTeamId,
}: Props & { myTeamId: string | null }) {
  const user = useSignedInUser();
  const concluded = match.concluded_at !== null;
  const playing = myTeamId !== null && (match.team1_id === myTeamId || match.team2_id === myTeamId);

  if (hasRole(user, "host")) {
    return (
      <StaffSelect
        label="Yayıncı"
        emptyLabel="— Yayıncı yok —"
        current={match.streamer}
        options={options}
        disabled={editor.busy}
        onSelect={(id) => editor.setStreamer(match.id, id)}
      />
    );
  }
  return (
    <span className="staff-control">
      {match.streamer ? (
        <strong>{match.streamer.username}</strong>
      ) : (
        <span className="muted">Henüz yok</span>
      )}
      {holdsRole(user, "streamer") && !concluded && !match.streamer && (
        <button
          type="button"
          className="btn btn-primary btn-sm"
          disabled={editor.busy || playing}
          title={playing ? "Kendi maçını yayınlayamazsın" : undefined}
          onClick={() => editor.setStreamer(match.id, user.id)}
        >
          Yayıncı ol
        </button>
      )}
      {!concluded && match.streamer?.id === user.id && (
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          disabled={editor.busy}
          onClick={() => editor.setStreamer(match.id, null)}
        >
          Bırak
        </button>
      )}
    </span>
  );
}
