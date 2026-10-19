import { useEffect, useRef, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import type { BracketMatchUpdate } from "../../../api/bracket";
import type { BracketSide, MatchStage, NextMatch, Player, RefereeViewer, Stage } from "../../../api/types";
import { useSignedInUser } from "../../../auth/AuthContext";
import { hasRole } from "../../../auth/roles";
import { MatchStatus } from "../../../components/bracket/BracketList";
import { RefereeWatchers } from "../../../components/bracket/RefereeWatchers";
import { DiscordMention } from "../../../components/DiscordMention";
import { EditableNote } from "../../../components/EditableNote";
import { IconButton } from "../../../components/IconButton";
import { MatchPageLink } from "../../../components/MatchPageLink";
import {
  BOARD,
  compareByCode,
  hasMatchPage,
  isLive,
  isUndecided,
  matchCode,
  nextNumber,
  potentialRows,
  SIDE_LABELS,
  sourceLabel,
  type Source,
} from "../../../lib/bracket";
import { formatShortMatchTime, fromDateTimeLocal, toDateTimeLocal } from "../../../lib/format";
import { RoomsEditor } from "../MatchControls";
import { RefereeControl, StreamerControl } from "./StaffControls";
import type { BracketEditor, EditorMatch, EditorTeam } from "./useBracketEditor";

/** Where the panel stops sitting beside the bracket and slides over it as a drawer (see the CSS). */
const DRAWER_QUERY = "(max-width: 1100px)";

interface Props {
  match: EditorMatch;
  matches: EditorMatch[];
  teams: Map<string, EditorTeam>;
  sources: Map<string, [Source<EditorMatch> | null, Source<EditorMatch> | null]>;
  myTeamId: string | null;
  editor: BracketEditor;
  referees: Player[];
  streamers: Player[];
  /** Every stage, for the host's stage picker. */
  stages: Stage[];
  /** Who has the match's referee page open right now. */
  watchers: RefereeViewer[] | undefined;
  onClose: () => void;
}

export function MatchPanel({
  match,
  matches,
  teams,
  sources,
  myTeamId,
  editor,
  referees,
  streamers,
  stages,
  watchers,
  onClose,
}: Props) {
  const user = useSignedInUser();
  const isHost = hasRole(user, "host");
  const { busy } = editor;
  const code = matchCode(match);
  const concluded = match.concluded_at !== null;
  // Any referee can run any match (its own referee is the one Discord pings); hosts also
  // after it's concluded.
  const canRun = hasRole(user, "referee");
  const canEdit = isHost || (canRun && !concluded);
  const team = (id: string | null) => (id ? teams.get(id) : undefined);
  const slotSources = sources.get(match.id) ?? [null, null];

  // As a drawer the panel covers the bracket, so focus moves into it when it opens.
  const panelRef = useRef<HTMLElement>(null);
  useEffect(() => {
    if (window.matchMedia(DRAWER_QUERY).matches) panelRef.current?.focus({ preventScroll: true });
  }, []);

  // A score typed into the panel but not saved yet: "Maçı bitir" saves it and concludes with it,
  // rather than with the last saved one.
  const [draft, setDraft] = useState<[number | null, number | null] | null>(null);
  const [score1, score2] = draft ?? [match.team1_score, match.team2_score];

  const concludeBlocker = match.rooms.length === 0
    ? "Önce maç linkini ekle"
    : !match.team1_id || !match.team2_id
      ? "İki takım da belli olmalı"
      : score1 === null || score2 === null || score1 === score2
        ? "Önce kazananı belli eden bir skor gir"
        : null;

  function confirmConclude() {
    const [t1, t2] = [team(match.team1_id), team(match.team2_id)];
    const team1Won = (score1 ?? 0) > (score2 ?? 0);
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
    const score = `Skor: ${score1} – ${score2}${draft ? " (kaydedilip bitirilir)" : ""}`;
    if (!window.confirm(`${code} bitti olarak işaretlensin mi?\n${score}${detail}`)) return;
    if (draft) editor.concludeWithScore(match.id, score1, score2);
    else editor.conclude(match.id);
  }

  return (
    <>
      {/* Behind the drawer on phones: a tap beside it closes it. */}
      <div className="match-panel-scrim" aria-hidden="true" onClick={onClose} />
      <aside
        ref={panelRef}
        className="match-panel"
        aria-label={`${code} maçı`}
        tabIndex={-1}
        onKeyDown={(e) => {
          // Escape in a field belongs to the field (cancelling an edit), not to the panel.
          if (e.key !== "Escape" || (e.target as Element).closest("input, textarea, select")) return;
          onClose();
        }}
      >
        <header className="match-panel-header">
          <h2 className={`match-panel-code is-${match.side}`}>{code}</h2>
          {isUndecided(match) ? (
            <span className="badge">Kesinleşmedi</span>
          ) : (
            <MatchStatus match={match} now={new Date().toISOString()} />
          )}
          <RefereeWatchers viewers={watchers} />
          {hasMatchPage(match) && <MatchPageLink matchId={match.id} live={isLive(match)} variant="row" />}
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

        {isUndecided(match) && (
          <PotentialList
            match={match}
            team={team}
            editor={editor}
            referees={referees}
            streamers={streamers}
            myTeamId={myTeamId}
          />
        )}

        <dl className="match-panel-rows">
          <div>
            <dt>Stage</dt>
            <dd>
              <StageName stage={match.stage} />
            </dd>
          </div>
          <div>
            <dt>Hakem</dt>
            <dd>
              <RefereeControl match={match} editor={editor} options={referees} />
            </dd>
          </div>
          <div>
            <dt>Yayıncı</dt>
            <dd>
              <StreamerControl match={match} editor={editor} options={streamers} myTeamId={myTeamId} />
            </dd>
          </div>
          <div>
            <dt>Skor</dt>
            <dd>
              {canRun && !concluded ? (
                <ScoreEditor
                  // A fresh editor for another match or a newly saved score (a referee's live
                  // save included), so it never shows a stale draft.
                  key={`${match.id}:${match.team1_score}:${match.team2_score}`}
                  match={match}
                  names={[team(match.team1_id)?.name ?? "1. takım", team(match.team2_id)?.name ?? "2. takım"]}
                  disabled={busy}
                  onSave={(a, b) => editor.setScore(match.id, a, b)}
                  onDraft={setDraft}
                />
              ) : match.forfeit_slot !== null ? (
                <span className="match-panel-score">
                  {match.forfeit_slot === 1 ? "FF : W" : "W : FF"}{" "}
                  <span className="muted">
                    (hükmen, {team(match.forfeit_slot === 1 ? match.team1_id : match.team2_id)?.name ?? "?"} çekildi)
                  </span>
                </span>
              ) : (
                <span className="match-panel-score">
                  {match.team1_score ?? "–"} : {match.team2_score ?? "–"}
                </span>
              )}
            </dd>
          </div>
          <div>
            <dt>{match.rooms.length > 1 ? "Odalar" : "Maç linki"}</dt>
            <dd>
              <RoomsEditor
                rooms={match.rooms}
                editable={canEdit}
                disabled={busy}
                onAdd={(link) => editor.addRoom(match.id, link)}
                onRemove={(mpId) => editor.removeRoom(match.id, mpId)}
              />
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
            <Link to={`/admin/referee/${match.id}`} className="btn btn-secondary btn-sm">
              Hakem odası
            </Link>
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
            {!concluded && match.team1_id && match.team2_id && (
              <ForfeitButtons
                names={[team(match.team1_id)?.name ?? "1. takım", team(match.team2_id)?.name ?? "2. takım"]}
                disabled={busy}
                onForfeit={(slot) => editor.forfeit(match.id, slot)}
              />
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
              stages={stages}
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
    </>
  );
}

interface PotentialListProps {
  match: EditorMatch;
  team: (id: string | null) => EditorTeam | undefined;
  editor: BracketEditor;
  referees: Player[];
  streamers: Player[];
  myTeamId: string | null;
}

/**
 * The pairings an undecided match may turn out to be, each with the time its teams agreed on and its own
 * referee and streamer, which are taken like a match's.
 */
function PotentialList({ match, team, editor, referees, streamers, myTeamId }: PotentialListProps) {
  const rows = potentialRows(match);
  return (
    <section className="match-panel-potentials" aria-labelledby="match-panel-potentials">
      <h3 id="match-panel-potentials">
        Olası eşleşmeler <span className="num">{rows.length}</span>
      </h3>
      <p className="muted match-panel-hint">
        Bu haftaki maçlar bitince biri bu maç olur ve saatiyle hakemini ona taşır; diğerleri kalkar. Maçın saati,
        hakemi ve yayıncısı, değiştirilmemiş eşleşmelere de geçer.
      </p>
      <ul className="match-panel-potential-list">
        {rows.map((p) => (
          <li key={p.id} className="match-panel-potential">
            <div className="match-panel-potential-head">
              <span className="match-panel-potential-teams">
                {team(p.team1_id)?.name ?? "?"} <span className="muted">vs</span> {team(p.team2_id)?.name ?? "?"}
              </span>
              <span className="match-panel-potential-time">
                {p.starts_at ? formatShortMatchTime(p.starts_at) : <span className="muted">Saat yok</span>}
              </span>
            </div>
            <dl className="match-panel-rows">
              <div>
                <dt>Hakem</dt>
                <dd>
                  <RefereeControl match={p} editor={editor} options={referees} />
                </dd>
              </div>
              <div>
                <dt>Yayıncı</dt>
                <dd>
                  <StreamerControl match={p} editor={editor} options={streamers} myTeamId={myTeamId} />
                </dd>
              </div>
            </dl>
          </li>
        ))}
      </ul>
    </section>
  );
}

// Remounts the form whenever the saved values change, so it never shows stale drafts.
function structureKey(m: EditorMatch): string {
  const link = (n: NextMatch | null) => (n ? `${n.match_id}/${n.slot}` : "");
  return [
    m.id,
    m.side,
    m.number,
    m.starts_at,
    m.team1_id,
    m.team2_id,
    link(m.winner_to),
    link(m.loser_to),
    m.stage?.id,
  ].join("|");
}

/** The match's stage, linking to its map pool once that's public. */
function StageName({ stage }: { stage: MatchStage | null }) {
  if (!stage) return <span className="muted">Belirlenmedi</span>;
  if (!stage.published) {
    return (
      <span className="match-panel-person">
        <strong>{stage.name}</strong>
        <span className="badge badge-warning">Mappool taslak</span>
      </span>
    );
  }
  return (
    <Link to={`/mappool?stage=${stage.id}`}>
      <strong>{stage.name}</strong> mappool'u
    </Link>
  );
}

interface ScoreEditorProps {
  match: EditorMatch;
  names: [string, string];
  disabled: boolean;
  onSave: (team1: number | null, team2: number | null) => void;
  /** The typed score while it differs from the saved one, null otherwise. */
  onDraft: (draft: [number | null, number | null] | null) => void;
}

function ScoreEditor({ match, names, disabled, onSave, onDraft }: ScoreEditorProps) {
  const [a, setA] = useState(match.team1_score?.toString() ?? "");
  const [b, setB] = useState(match.team2_score?.toString() ?? "");
  const parse = (v: string) => (v.trim() === "" ? null : Number(v));
  const changed = parse(a) !== match.team1_score || parse(b) !== match.team2_score;
  // Reported whenever the inputs change.
  useEffect(() => {
    onDraft(changed ? [parse(a), parse(b)] : null);
  }, [a, b, changed, onDraft]);
  // The panel forgets the draft when this editor goes away (another match, a saved score).
  useEffect(() => () => onDraft(null), [onDraft]);

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
  stages: Stage[];
  disabled: boolean;
  onSave: (update: BracketMatchUpdate) => void;
}

const linkValue = (n: NextMatch | null) => (n ? `${n.match_id}:${n.slot}` : "");

function parseLink(value: string): NextMatch | null {
  if (!value) return null;
  const [match_id, slot] = value.split(":");
  return { match_id, slot: slot === "2" ? 2 : 1 };
}

/** The host's edits: code, stage, time, teams and where the winner and loser go. */
function StructureForm({ match, matches, teams, sources, stages, disabled, onSave }: StructureFormProps) {
  const [side, setSide] = useState(match.side);
  const [number, setNumber] = useState(String(match.number));
  const [startsAt, setStartsAt] = useState(match.starts_at ? toDateTimeLocal(match.starts_at) : "");
  const [team1, setTeam1] = useState(match.team1_id ?? "");
  const [team2, setTeam2] = useState(match.team2_id ?? "");
  const [winnerTo, setWinnerTo] = useState(linkValue(match.winner_to));
  const [loserTo, setLoserTo] = useState(linkValue(match.loser_to));
  const [stageId, setStageId] = useState(match.stage?.id ?? "");
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
      stage_id: stageId || null,
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
        <span>Stage</span>
        <select
          className="select select-sm"
          value={stageId}
          disabled={disabled}
          onChange={(e) => setStageId(e.target.value)}
        >
          <option value="">— Stage yok —</option>
          {stages.map((s) => (
            <option key={s.id} value={s.id}>
              {s.name}
            </option>
          ))}
        </select>
      </label>
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

/**
 * Concludes the match by forfeit when a team didn't show up or it couldn't be finished: pick the
 * team that forfeited and the other one moves on. Opens on a click, then asks once more.
 */
function ForfeitButtons({
  names,
  disabled,
  onForfeit,
}: {
  names: [string, string];
  disabled: boolean;
  onForfeit: (slot: 1 | 2) => void;
}) {
  const [open, setOpen] = useState(false);
  if (!open) {
    return (
      <button
        type="button"
        className="btn btn-danger btn-sm"
        disabled={disabled}
        title="Bir takım gelmediyse ya da maç tamamlanamadıysa"
        onClick={() => setOpen(true)}
      >
        Hükmen…
      </button>
    );
  }
  return (
    <span className="match-panel-forfeit" role="group" aria-label="Hükmen bitir: çekilen takım">
      <span className="match-panel-forfeit-label">Çekilen takım:</span>
      {names.map((name, i) => (
        <button
          key={i}
          type="button"
          className="btn btn-danger btn-sm"
          disabled={disabled}
          onClick={() => {
            if (window.confirm(`${name} çekilmiş sayılsın mı? Maç hükmen biter ve ${names[1 - i]} kazanır.`)) {
              onForfeit((i + 1) as 1 | 2);
            }
          }}
        >
          {name}
        </button>
      ))}
      <button type="button" className="btn btn-secondary btn-sm" onClick={() => setOpen(false)}>
        Vazgeç
      </button>
    </span>
  );
}
