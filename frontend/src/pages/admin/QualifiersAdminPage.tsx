import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../../api/client";
import {
  addMatchRoom,
  assignTeam,
  concludeMatch,
  createLobby,
  deleteLobby,
  fetchQualifiers,
  fetchReferees,
  removeMatchRoom,
  reopenMatch,
  rescheduleLobby,
  setMatchNotes,
  setReferee,
  unassignTeam,
} from "../../api/matches";
import type { AdminTeamPlayer, Player, StaffLobby, StaffQualifiers, Team } from "../../api/types";
import { useSignedInUser } from "../../auth/AuthContext";
import { hasRole } from "../../auth/roles";
import { DiscordMention } from "../../components/DiscordMention";
import { EditableNote } from "../../components/EditableNote";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { IconButton } from "../../components/IconButton";
import { LobbyCard } from "../../components/LobbyCard";
import { formatMatchTime, fromDateTimeLocal, toDateTimeLocal } from "../../lib/format";
import { teamListPath } from "../../lib/teamLink";
import { RoomsEditor, StaffSelect } from "./MatchControls";
import "./QualifiersAdminPage.css";

type Run = (action: () => Promise<void>) => Promise<void>;

export function QualifiersAdminPage() {
  const user = useSignedInUser();
  const isHost = hasRole(user, "host");
  const [data, setData] = useState<StaffQualifiers | null>(null);
  const [referees, setReferees] = useState<Player[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [mineOnly, setMineOnly] = useState(false);
  const [newTime, setNewTime] = useState("");

  const reload = useCallback(async () => setData(await fetchQualifiers()), []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
    if (isHost) fetchReferees().then(setReferees).catch((e) => setError(errorMessage(e)));
  }, [reload, isHost]);

  const run: Run = async (action) => {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    }
    try {
      await reload();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  };

  // Notes save in the background so typing never waits on the server.
  function saveNotes(lobby: StaffLobby, notes: string) {
    setData((d) => d && { ...d, lobbies: d.lobbies.map((l) => (l.id === lobby.id ? { ...l, referee_notes: notes } : l)) });
    setMatchNotes(lobby.id, notes).catch((e) => {
      setError(errorMessage(e));
      reload().catch(() => {});
    });
  }

  function addLobby(e: FormEvent) {
    e.preventDefault();
    if (!newTime) return;
    run(async () => {
      await createLobby(fromDateTimeLocal(newTime));
      setNewTime("");
    });
  }

  const lobbies = data?.lobbies ?? [];
  // Numbered by place in the full list, so filtering doesn't renumber them.
  const numbered = lobbies.map((lobby, i) => ({ lobby, number: i + 1 }));
  const shown = mineOnly ? numbered.filter(({ lobby }) => lobby.referee?.id === user.id) : numbered;
  const unrefereed = lobbies.filter((l) => !l.referee && !l.concluded_at).length;

  return (
    <section className="panel qualifiers-admin">
      <div className="page-header">
        <h1>Elemeler</h1>
        {data && <span className="badge">{lobbies.length} lobi</span>}
        {unrefereed > 0 && (
          <span className="badge badge-warning" title="Henüz hakemi olmayan lobiler">
            {unrefereed} hakemsiz
          </span>
        )}
        {data && (
          <div className="page-header-aside">
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              aria-pressed={mineOnly}
              onClick={() => setMineOnly((on) => !on)}
            >
              Benim maçlarım
            </button>
            {isHost && (
              <form className="inline-form" onSubmit={addLobby}>
                <input
                  className="input input-sm"
                  type="datetime-local"
                  aria-label="Yeni lobinin başlangıç saati"
                  value={newTime}
                  onChange={(e) => setNewTime(e.target.value)}
                />
                <button type="submit" className="btn btn-primary btn-sm" disabled={busy || !newTime}>
                  Lobi ekle
                </button>
              </form>
            )}
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!data && !error && <p className="status">Yükleniyor…</p>}

      {data && isHost && data.unassigned.length > 0 && (
        <div className="qualifiers-unassigned">
          <h2>
            Lobisi olmayan takımlar <span className="badge">{data.unassigned.length}</span>
          </h2>
          <ul>
            {data.unassigned.map((team) => (
              <li key={team.id}>
                <Link className="qualifiers-unassigned-name" to={teamListPath(team.id)}>
                  {team.name}
                </Link>
                <span className="muted">{team.players.map((p) => p.username).join(" & ")}</span>
                <LobbySelect
                  label="Lobiye ata…"
                  team={team}
                  lobbies={lobbies}
                  capacity={data.capacity}
                  disabled={busy}
                  onSelect={(id) => run(() => assignTeam(team.id, id))}
                />
              </li>
            ))}
          </ul>
        </div>
      )}

      {data && lobbies.length === 0 && (
        <p className="status">
          Henüz eleme lobisi yok.{isHost && " Yukarıdan bir başlangıç saati seçip ilk lobiyi ekle."}
        </p>
      )}
      {data && mineOnly && shown.length === 0 && lobbies.length > 0 && (
        <p className="status">Hakemi olduğun bir lobi yok.</p>
      )}
      {data && shown.length > 0 && (
        <div className="lobby-grid">
          {shown.map(({ lobby, number }) => (
            <StaffLobbyCard
              key={lobby.id}
              lobby={lobby}
              number={number}
              data={data}
              userId={user.id}
              isHost={isHost}
              referees={referees}
              busy={busy}
              run={run}
              onNotes={(notes) => saveNotes(lobby, notes)}
              onStart={() => reload().catch((e) => setError(errorMessage(e)))}
            />
          ))}
        </div>
      )}
    </section>
  );
}

interface StaffLobbyCardProps {
  lobby: StaffLobby;
  number: number;
  data: StaffQualifiers;
  userId: string;
  isHost: boolean;
  referees: Player[];
  busy: boolean;
  run: Run;
  onNotes: (notes: string) => void;
  onStart: () => void;
}

function StaffLobbyCard({
  lobby,
  number,
  data,
  userId,
  isHost,
  referees,
  busy,
  run,
  onNotes,
  onStart,
}: StaffLobbyCardProps) {
  const concluded = lobby.concluded_at !== null;
  const isMine = lobby.referee?.id === userId;
  // The match's referee runs it; hosts can run any match, also after it's concluded.
  const canRun = isHost || isMine;
  const canEdit = isHost || (isMine && !concluded);

  return (
    <LobbyCard
      lobby={lobby}
      number={number}
      capacity={data.capacity}
      highlight={isMine}
      onStart={onStart}
      renderPlayerExtra={(p) => <DiscordMention player={p} fallback="Discord yok" />}
      renderTeamActions={
        isHost
          ? (team) => (
              <>
                <LobbySelect
                  label="Başka lobiye taşı"
                  compact
                  team={team}
                  current={lobby.id}
                  lobbies={data.lobbies}
                  capacity={data.capacity}
                  disabled={busy}
                  onSelect={(id) => run(() => assignTeam(team.id, id))}
                />
                <IconButton
                  icon="close"
                  label={`${team.name} takımını lobiden çıkar`}
                  tone="danger"
                  disabled={busy}
                  onClick={() => run(() => unassignTeam(team.id))}
                />
              </>
            )
          : undefined
      }
      actions={
        canRun && (
          <>
            {isHost && (
              <button
                type="button"
                className="btn btn-danger btn-sm lobby-delete"
                disabled={busy}
                onClick={() => {
                  const teams = lobby.teams.length > 0 ? ` İçindeki ${lobby.teams.length} takım lobisiz kalır.` : "";
                  if (window.confirm(`Lobi ${number} silinsin mi?${teams}`)) run(() => deleteLobby(lobby.id));
                }}
              >
                Lobiyi sil
              </button>
            )}
            <Link to={`/admin/referee/${lobby.id}`} className="btn btn-secondary btn-sm">
              Hakem odası
            </Link>
            {canRun && !concluded && (
              <button
                type="button"
                className="btn btn-primary btn-sm"
                disabled={busy || lobby.rooms.length === 0}
                title={lobby.rooms.length > 0 ? undefined : "Önce maç linkini ekle"}
                onClick={() => {
                  if (window.confirm(`Lobi ${number} bitti olarak işaretlensin mi?`)) run(() => concludeMatch(lobby.id));
                }}
              >
                Maçı bitir
              </button>
            )}
            {isHost && concluded && (
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                disabled={busy}
                onClick={() => run(() => reopenMatch(lobby.id))}
              >
                Yeniden aç
              </button>
            )}
          </>
        )
      }
    >
      <dl className="staff-rows">
        {isHost && (
          <div>
            <dt>Saat</dt>
            <dd>
              <ScheduleEditor
                value={lobby.starts_at}
                disabled={busy}
                onSave={(iso) => run(() => rescheduleLobby(lobby.id, iso))}
              />
            </dd>
          </div>
        )}
        <div>
          <dt>Hakem</dt>
          <dd>
            {isHost ? (
              <StaffSelect
                label="Hakem"
                emptyLabel="— Hakem yok —"
                current={lobby.referee}
                options={referees}
                disabled={busy}
                onSelect={(id) => run(() => setReferee(lobby.id, id))}
              />
            ) : (
              <span className="staff-referee">
                {lobby.referee ? <strong>{lobby.referee.username}</strong> : <span className="muted">Henüz yok</span>}
                {!concluded && !lobby.referee && (
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    disabled={busy}
                    onClick={() => run(() => setReferee(lobby.id, userId))}
                  >
                    Hakemliği üstlen
                  </button>
                )}
                {!concluded && isMine && (
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    disabled={busy}
                    onClick={() => run(() => setReferee(lobby.id, null))}
                  >
                    Bırak
                  </button>
                )}
              </span>
            )}
          </dd>
        </div>
        <div>
          <dt>{lobby.rooms.length > 1 ? "Odalar" : "Maç linki"}</dt>
          <dd>
            <RoomsEditor
              rooms={lobby.rooms}
              editable={canEdit}
              disabled={busy}
              onAdd={(link) => run(() => addMatchRoom(lobby.id, link))}
              onRemove={(mpId) => run(() => removeMatchRoom(lobby.id, mpId))}
            />
          </dd>
        </div>
        <div>
          <dt>Notlar</dt>
          <dd>
            {canRun ? (
              <EditableNote
                value={lobby.referee_notes}
                label={`Lobi ${number} hakem notları`}
                placeholder="Ters giden bir şey olursa not al (sadece yetkililer görür)"
                isPrivate
                onSave={onNotes}
              />
            ) : (
              <span className={lobby.referee_notes ? "staff-notes" : "muted"}>{lobby.referee_notes || "—"}</span>
            )}
          </dd>
        </div>
      </dl>
    </LobbyCard>
  );
}

interface LobbySelectProps {
  label: string;
  team: Team<AdminTeamPlayer>;
  /** The lobby the team is in, left out of the options. */
  current?: string;
  lobbies: StaffLobby[];
  capacity: number;
  disabled: boolean;
  /** Shows only an icon; the label becomes its tooltip. */
  compact?: boolean;
  onSelect: (lobbyId: string) => void;
}

// Hosts can move a team into any lobby that has room, started or not.
function LobbySelect({ label, team, current, lobbies, capacity, disabled, compact, onSelect }: LobbySelectProps) {
  const select = (
    <select
      className={compact ? "icon-select-native" : "select select-sm"}
      value=""
      disabled={disabled}
      title={compact ? label : undefined}
      aria-label={`${team.name}: ${label}`}
      onChange={(e) => e.target.value && onSelect(e.target.value)}
    >
      <option value="">{label}</option>
      {lobbies.map((lobby, i) =>
        lobby.id === current ? null : (
          <option key={lobby.id} value={lobby.id} disabled={lobby.teams.length >= capacity}>
            Lobi {i + 1} · {formatMatchTime(lobby.starts_at)} ({lobby.teams.length}/{capacity})
          </option>
        ),
      )}
    </select>
  );
  if (!compact) return select;
  // An invisible native select over the icon, so it still opens the system picker.
  return (
    <span className={disabled ? "icon-select is-disabled" : "icon-select"}>
      <Icon name="swap" />
      {select}
    </span>
  );
}

interface ScheduleEditorProps {
  value: string;
  disabled: boolean;
  onSave: (next: string) => void;
}

function ScheduleEditor({ value, disabled, onSave }: ScheduleEditorProps) {
  const initial = toDateTimeLocal(value);
  const [draft, setDraft] = useState(initial);
  useEffect(() => setDraft(initial), [initial]);
  const changed = draft !== "" && draft !== initial;

  return (
    <form
      className="inline-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (changed) onSave(fromDateTimeLocal(draft));
      }}
    >
      <input
        className="input input-sm"
        type="datetime-local"
        aria-label="Başlangıç saati"
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
      />
      <button type="submit" className="btn btn-secondary btn-sm" disabled={disabled || !changed}>
        Kaydet
      </button>
    </form>
  );
}
