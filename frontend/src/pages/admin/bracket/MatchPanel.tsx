import { useState, type FormEvent } from "react";
import type { BracketSide, NextMatch, Player } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import { MatchStatus } from "../../../components/bracket/BracketList";
import { DiscordMention } from "../../../components/DiscordMention";
import { EditableNote } from "../../../components/EditableNote";
import { IconButton } from "../../../components/IconButton";
import {
  BOARD,
  compareByCode,
  matchCode,
  nextNumber,
  SIDE_LABELS,
  sourceLabel,
  type Source,
} from "../../../lib/bracket";
import { fromDateTimeLocal, toDateTimeLocal } from "../../../lib/format";
import { LinkEditor, StaffSelect } from "../MatchControls";
import type { BracketEditor, EditorMatch, EditorTeam } from "./useBracketEditor";

interface Props {
  match: EditorMatch;
  matches: EditorMatch[];
  teams: Map<string, EditorTeam>;
  sources: Map<string, [Source<EditorMatch> | null, Source<EditorMatch> | null]>;
  myTeamId: string | null;
  editor: BracketEditor;
  referees: Player[];
  streamers: Player[];
  onClose: () => void;
}

export function MatchPanel({ match, matches, teams, sources, myTeamId, editor, referees, streamers, onClose }: Props) {
  const user = useSignedInUser();
  const isHost = hasRole(user, "host");
  const isReferee = hasRole(user, "referee");
  const { busy } = editor;
  const code = matchCode(match);
  const concluded = match.concluded_at !== null;
  const isMine = match.referee?.id === user.id;
  // The match's referee runs it; hosts can run any match, also after it's concluded.
  const canRun = isHost || isMine;
  const canEdit = isHost || (isMine && !concluded);
  const team = (id: string | null) => (id ? teams.get(id) : undefined);
  const slotSources = sources.get(match.id) ?? [null, null];
  const playing = myTeamId !== null && (match.team1_id === myTeamId || match.team2_id === myTeamId);

  const concludeBlocker = !match.mp_url
    ? "Önce maç linkini ekle"
    : !match.team1_id || !match.team2_id
      ? "İki takım da belli olmalı"
      : match.team1_score === null || match.team2_score === null || match.team1_score === match.team2_score
        ? "Önce kazananı belli eden bir skor gir"
        : null;

  function confirmConclude() {
    const [t1, t2] = [team(match.team1_id), team(match.team2_id)];
    const team1Won = (match.team1_score ?? 0) > (match.team2_score ?? 0);
    const [winner, loser] = team1Won ? [t1, t2] : [t2, t1];
    const target = (next: NextMatch | null) => {
      const m = next && matches.find((x) => x.id === next.match_id);
      return m ? matchCode(m) : null;
    };
    const moves = [
      winner && target(match.winner_to) && `${winner.name} → ${target(match.winner_to)}`,
      loser && target(match.loser_to) && `${loser.name} → ${target(match.loser_to)}`,
    ].filter(Boolean);
    const detail = moves.length > 0 ? `\n\n${moves.join("\n")}` : "";
    if (window.confirm(`${code} bitti olarak işaretlensin mi?${detail}`)) editor.conclude(match.id);
  }

  return (
    <aside className="match-panel" aria-label={`${code} maçı`}>
      <header className="match-panel-header">
        <h2 className={`match-panel-code is-${match.side}`}>{code}</h2>
        <MatchStatus match={match} now={new Date().toISOString()} />
        <IconButton icon="close" label="Paneli kapat" onClick={onClose} className="match-panel-close" />
      </header>

      <div className="match-panel-teams">
        {[match.team1_id, match.team2_id].map((id, i) => {
          const t = team(id);
          const source = slotSources[i];
          return (
            <div key={i} className="match-panel-team">
              <span className="match-panel-team-name">
                {t ? t.name : <span className="muted">{source ? sourceLabel(source) : "Belirlenmedi"}</span>}
              </span>
              {t && (
                <span className="match-panel-players">
                  {t.players.map((p) => (
                    <span key={p.id}>
                      {p.username}
                      {"discord_username" in p && (
                        <>
                          {" "}
                          <DiscordMention player={p} fallback="" />
                        </>
                      )}
                    </span>
                  ))}
                </span>
              )}
            </div>
          );
        })}
      </div>

      <dl className="match-panel-rows">
        <div>
          <dt>Hakem</dt>
          <dd>
            {isHost ? (
              <StaffSelect
                label="Hakem"
                emptyLabel="— Hakem yok —"
                current={match.referee}
                options={referees}
                disabled={busy}
                onSelect={(id) => editor.setReferee(match.id, id)}
              />
            ) : (
              <span className="match-panel-person">
                {match.referee ? <strong>{match.referee.username}</strong> : <span className="muted">Henüz yok</span>}
                {isReferee && !concluded && !match.referee && (
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    disabled={busy}
                    onClick={() => editor.setReferee(match.id, user.id)}
                  >
                    Hakemliği üstlen
                  </button>
                )}
                {!concluded && isMine && (
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    disabled={busy}
                    onClick={() => editor.setReferee(match.id, null)}
                  >
                    Bırak
                  </button>
                )}
              </span>
            )}
          </dd>
        </div>
        <div>
          <dt>Yayıncı</dt>
          <dd>
            {isHost ? (
              <StaffSelect
                label="Yayıncı"
                emptyLabel="— Yayıncı yok —"
                current={match.streamer}
                options={streamers}
                disabled={busy}
                onSelect={(id) => editor.setStreamer(match.id, id)}
              />
            ) : (
              <span className="match-panel-person">
                {match.streamer ? <strong>{match.streamer.username}</strong> : <span className="muted">Henüz yok</span>}
                {!concluded && !match.streamer && (
                  <button
                    type="button"
                    className="btn btn-primary btn-sm"
                    disabled={busy || playing}
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
                    disabled={busy}
                    onClick={() => editor.setStreamer(match.id, null)}
                  >
                    Bırak
                  </button>
                )}
              </span>
            )}
          </dd>
        </div>
        <div>
          <dt>Skor</dt>
          <dd>
            {canRun && !concluded ? (
              <ScoreEditor
                key={`${match.team1_score}:${match.team2_score}`}
                match={match}
                names={[team(match.team1_id)?.name ?? "1. takım", team(match.team2_id)?.name ?? "2. takım"]}
                disabled={busy}
                onSave={(a, b) => editor.setScore(match.id, a, b)}
              />
            ) : (
              <span className="match-panel-score">
                {match.team1_score ?? "–"} : {match.team2_score ?? "–"}
              </span>
            )}
          </dd>
        </div>
        <div>
          <dt>Maç linki</dt>
          <dd>
            {canEdit ? (
              <LinkEditor value={match.mp_url} disabled={busy} onSave={(link) => editor.setLink(match.id, link)} />
            ) : match.mp_url ? (
              <a href={match.mp_url} target="_blank" rel="noopener noreferrer">
                {match.mp_url.replace(/^https:\/\//, "")}
              </a>
            ) : (
              <span className="muted">—</span>
            )}
          </dd>
        </div>
        {match.referee_notes !== undefined && (
          <div>
            <dt>Notlar</dt>
            <dd>
              {canRun ? (
                <EditableNote
                  value={match.referee_notes}
                  label={`${code} hakem notları`}
                  placeholder="Ters giden bir şey olursa not al (sadece yetkililer görür)"
                  isPrivate
                  onSave={(notes) => editor.saveNotes(match.id, notes)}
                />
              ) : (
                <span className={match.referee_notes ? "match-panel-notes" : "muted"}>
                  {match.referee_notes || "—"}
                </span>
              )}
            </dd>
          </div>
        )}
      </dl>

      {canRun && (
        <div className="match-panel-actions">
          {!concluded && (
            <button
              type="button"
              className="btn btn-primary btn-sm"
              disabled={busy || concludeBlocker !== null}
              title={concludeBlocker ?? undefined}
              onClick={confirmConclude}
            >
              Maçı bitir
            </button>
          )}
          {isHost && concluded && (
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy}
              onClick={() => editor.reopen(match.id)}
            >
              Yeniden aç
            </button>
          )}
        </div>
      )}

      {isHost && (
        <>
          <StructureForm
            key={structureKey(match)}
            match={match}
            matches={matches}
            teams={teams}
            sources={sources}
            disabled={busy}
            onSave={(update) => editor.update(match.id, update)}
          />
          <PositionForm
            key={`${match.pos_x}:${match.pos_y}`}
            match={match}
            disabled={busy}
            onMove={(x, y) => editor.move(match.id, x, y)}
          />
          <div className="match-panel-actions">
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={busy}
              onClick={() => {
                if (window.confirm(`${code} silinsin mi? Bu maçın ileri taşıdığı takımlar yerinde kalır.`)) {
                  editor.remove(match.id);
                  onClose();
                }
              }}
            >
              Maçı sil
            </button>
          </div>
        </>
      )}
    </aside>
  );
}

// Remounts the form whenever the saved values change, so it never shows stale drafts.
function structureKey(m: EditorMatch): string {
  const link = (n: NextMatch | null) => (n ? `${n.match_id}/${n.slot}` : "");
  return [m.id, m.side, m.number, m.starts_at, m.team1_id, m.team2_id, link(m.winner_to), link(m.loser_to)].join("|");
}

interface ScoreEditorProps {
  match: EditorMatch;
  names: [string, string];
  disabled: boolean;
  onSave: (team1: number | null, team2: number | null) => void;
}

function ScoreEditor({ match, names, disabled, onSave }: ScoreEditorProps) {
  const [a, setA] = useState(match.team1_score?.toString() ?? "");
  const [b, setB] = useState(match.team2_score?.toString() ?? "");
  const parse = (v: string) => (v.trim() === "" ? null : Number(v));
  const changed = parse(a) !== match.team1_score || parse(b) !== match.team2_score;

  return (
    <form
      className="inline-form match-panel-score-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (changed) onSave(parse(a), parse(b));
      }}
    >
      <input
        className="input input-sm"
        type="number"
        min={0}
        max={99}
        aria-label={`${names[0]} skoru`}
        value={a}
        onChange={(e) => setA(e.target.value)}
      />
      <span aria-hidden="true">:</span>
      <input
        className="input input-sm"
        type="number"
        min={0}
        max={99}
        aria-label={`${names[1]} skoru`}
        value={b}
        onChange={(e) => setB(e.target.value)}
      />
      <button type="submit" className="btn btn-secondary btn-sm" disabled={disabled || !changed}>
        Kaydet
      </button>
    </form>
  );
}

interface StructureFormProps {
  match: EditorMatch;
  matches: EditorMatch[];
  teams: Map<string, EditorTeam>;
  sources: Props["sources"];
  disabled: boolean;
  onSave: (update: {
    side: BracketSide;
    number: number;
    starts_at: string | null;
    team1_id: string | null;
    team2_id: string | null;
    winner_to: NextMatch | null;
    loser_to: NextMatch | null;
  }) => void;
}

const linkValue = (n: NextMatch | null) => (n ? `${n.match_id}:${n.slot}` : "");

function parseLink(value: string): NextMatch | null {
  if (!value) return null;
  const [match_id, slot] = value.split(":");
  return { match_id, slot: slot === "2" ? 2 : 1 };
}

/** The host's edits: code, time, teams and where the winner and loser go. */
function StructureForm({ match, matches, teams, sources, disabled, onSave }: StructureFormProps) {
  const [side, setSide] = useState(match.side);
  const [number, setNumber] = useState(String(match.number));
  const [startsAt, setStartsAt] = useState(match.starts_at ? toDateTimeLocal(match.starts_at) : "");
  const [team1, setTeam1] = useState(match.team1_id ?? "");
  const [team2, setTeam2] = useState(match.team2_id ?? "");
  const [winnerTo, setWinnerTo] = useState(linkValue(match.winner_to));
  const [loserTo, setLoserTo] = useState(linkValue(match.loser_to));
  const concluded = match.concluded_at !== null;

  const teamOptions = [...teams.values()].sort((a, b) => a.name.localeCompare(b.name, "tr"));
  const others = matches.filter((m) => m.id !== match.id).sort(compareByCode);

  // A new side gets the next free number there; switching back restores the old one.
  function changeSide(next: BracketSide) {
    setSide(next);
    setNumber(String(next === match.side ? match.number : nextNumber(matches, next)));
  }

  function submit(e: FormEvent) {
    e.preventDefault();
    onSave({
      side,
      number: Number(number),
      starts_at: startsAt ? fromDateTimeLocal(startsAt) : null,
      team1_id: team1 || null,
      team2_id: team2 || null,
      winner_to: parseLink(winnerTo),
      loser_to: parseLink(loserTo),
    });
  }

  function targetSelect(label: string, value: string, setValue: (v: string) => void, outcome: "winner" | "loser") {
    return (
      <label className="match-form-field">
        <span>{label}</span>
        <select
          className="select select-sm"
          value={value}
          disabled={disabled || concluded}
          onChange={(e) => setValue(e.target.value)}
        >
          <option value="">— Hiçbir yere —</option>
          {others.map((m) =>
            ([1, 2] as const).map((slot) => {
              const feeder = sources.get(m.id)?.[slot - 1];
              const takenByOther = feeder && !(feeder.match.id === match.id && feeder.outcome === outcome);
              return (
                <option key={`${m.id}:${slot}`} value={`${m.id}:${slot}`} disabled={!!takenByOther}>
                  {matchCode(m)} · {slot}. takım{takenByOther ? ` (${sourceLabel(feeder)})` : ""}
                </option>
              );
            }),
          )}
        </select>
      </label>
    );
  }

  function teamSelect(label: string, value: string, setValue: (v: string) => void, other: string) {
    return (
      <label className="match-form-field">
        <span>{label}</span>
        <select
          className="select select-sm"
          value={value}
          disabled={disabled || concluded}
          onChange={(e) => setValue(e.target.value)}
        >
          <option value="">— Boş —</option>
          {teamOptions.map((t) => (
            <option key={t.id} value={t.id} disabled={t.id === other}>
              {t.name}
            </option>
          ))}
        </select>
      </label>
    );
  }

  return (
    <form className="match-form" onSubmit={submit}>
      <h3>Maçı düzenle</h3>
      <div className="match-form-row">
        <label className="match-form-field">
          <span>Taraf</span>
          <select
            className="select select-sm"
            value={side}
            disabled={disabled}
            onChange={(e) => changeSide(e.target.value as BracketSide)}
          >
            {(["winners", "losers"] as const).map((s) => (
              <option key={s} value={s}>
                {SIDE_LABELS[s]}
              </option>
            ))}
          </select>
        </label>
        <label className="match-form-field match-form-number">
          <span>Numara</span>
          <input
            className="input input-sm"
            type="number"
            min={1}
            required
            value={number}
            disabled={disabled}
            onChange={(e) => setNumber(e.target.value)}
          />
        </label>
      </div>
      <label className="match-form-field">
        <span>Zaman</span>
        <span className="match-form-inline">
          <input
            className="input input-sm"
            type="datetime-local"
            value={startsAt}
            disabled={disabled}
            onChange={(e) => setStartsAt(e.target.value)}
          />
          {startsAt && (
            <IconButton icon="close" label="Zamanı kaldır" disabled={disabled} onClick={() => setStartsAt("")} />
          )}
        </span>
      </label>
      {teamSelect("1. takım", team1, setTeam1, team2)}
      {teamSelect("2. takım", team2, setTeam2, team1)}
      {targetSelect("Kazanan gider", winnerTo, setWinnerTo, "winner")}
      {targetSelect("Kaybeden gider", loserTo, setLoserTo, "loser")}
      {concluded && (
        <p className="match-form-hint">Takımları ve bağlantıları değiştirmek için önce maçı yeniden aç.</p>
      )}
      <button type="submit" className="btn btn-primary btn-sm" disabled={disabled}>
        Kaydet
      </button>
    </form>
  );
}

interface PositionFormProps {
  match: EditorMatch;
  disabled: boolean;
  onMove: (x: number, y: number) => void;
}

/** The alternative to dragging: the card's top left corner in board units. */
function PositionForm({ match, disabled, onMove }: PositionFormProps) {
  const [x, setX] = useState(String(match.pos_x));
  const [y, setY] = useState(String(match.pos_y));
  const changed = Number(x) !== match.pos_x || Number(y) !== match.pos_y;

  return (
    <form
      className="match-form"
      onSubmit={(e) => {
        e.preventDefault();
        if (changed) onMove(Number(x), Number(y));
      }}
    >
      <h3>Konum</h3>
      <div className="match-form-row">
        <label className="match-form-field">
          <span>X</span>
          <input
            className="input input-sm"
            type="number"
            required
            value={x}
            onChange={(e) => setX(e.target.value)}
          />
        </label>
        <label className="match-form-field">
          <span>Y</span>
          <input
            className="input input-sm"
            type="number"
            required
            value={y}
            onChange={(e) => setY(e.target.value)}
          />
        </label>
        <button type="submit" className="btn btn-secondary btn-sm" disabled={disabled || !changed}>
          Taşı
        </button>
      </div>
      <p className="match-form-hint">
        Izgara birimiyle; bir kart {BOARD.cardW}×{BOARD.cardH} birim. Seçili kartı ok tuşlarıyla da kaydırabilirsin
        (Shift ile 5 birim).
      </p>
    </form>
  );
}
