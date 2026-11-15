import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../../api/client";
import {
  makeBracket,
  fetchSeeding,
  removeBracket,
  resetSeeding,
  saveSeeding,
} from "../../api/seeding";
import type { BracketFormat, Seeding, SeedTeam } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { IconButton } from "../../components/IconButton";
import { useLiveBracket } from "../../live/LiveContext";
import { useDragReorder } from "../../lib/dragAndDrop";
import { RANK_WARNING_LABELS } from "../../lib/eligibility";
import { moveBy, moveToTarget } from "../../lib/reorder";
import { bracketWeeks, defaultStages, FORMAT_LABELS, FORMAT_TEAMS } from "../../lib/seeding";
import { useAction } from "../../lib/useAction";
import { ZValue } from "../../components/ZValue";
import "./SeedingAdminPage.css";

const FORMATS: BracketFormat[] = ["round_of_32", "round_of_16"];

const score = new Intl.NumberFormat("tr-TR");

/**
 * /admin/seeding: the qualifier's standings as the bracket's seeding. Hosts rearrange it by hand if
 * they need to (drag, the arrows, or a seed typed in), then make the double-elimination bracket from
 * its top 32 or 16 in one go; the teams under the cut are out. Once the bracket has matches the
 * seeding is its record and stays as it is, and the bracket can only be taken down until a match of
 * it is played.
 */
export function SeedingAdminPage() {
  const [data, setData] = useState<Seeding | null>(null);
  const [format, setFormat] = useState<BracketFormat>("round_of_32");
  // Each week's stage, once the host picked one; until then the default for the format.
  const [picked, setPicked] = useState<Partial<Record<BracketFormat, (string | null)[]>>>({});

  const load = useCallback(async () => setData(await fetchSeeding()), []);
  const { busy, error, setError, run } = useAction(load);

  useEffect(() => {
    load().catch((e) => setError(errorMessage(e)));
  }, [load, setError]);
  // Another host making or taking down the bracket.
  useLiveBracket(
    () => {},
    () => void load().catch(() => {}),
  );

  const teams = useMemo(() => data?.teams ?? [], [data]);
  const ids = useMemo(() => teams.map((t) => t.id), [teams]);
  const locked = (data?.matches ?? 0) > 0;
  const cut = FORMAT_TEAMS[format];
  const weeks = bracketWeeks(format);
  const stageIds = picked[format] ?? (data ? defaultStages(format, data.stages) : []);

  function applyOrder(next: string[] | null) {
    if (!next || !data) return;
    const byId = new Map(teams.map((t) => [t.id, t]));
    setData({ ...data, teams: next.map((id) => byId.get(id)!), custom: true });
    void run(() => saveSeeding(next));
  }

  const dragProps = useDragReorder((dragged, target) =>
    applyOrder(moveToTarget(ids, dragged, target)),
  );

  // A seed typed into a row's figure moves the team there.
  function moveToSeed(team: SeedTeam, value: string) {
    const to = Math.min(Math.max(Math.round(Number(value)), 1), ids.length) - 1;
    const from = ids.indexOf(team.id);
    if (!Number.isFinite(to) || to === from) return;
    applyOrder(moveBy(ids, team.id, to - from));
  }

  const ranked = teams.filter((t) => t.qualifier).length;
  const out = Math.max(teams.length - cut, 0);
  const blockers: string[] = [];
  if (data && !data.qualifier_stage_id)
    blockers.push("Elemelerin stage'i seçilmemiş (Stage'ler sekmesi).");
  if (data && data.lobbies_not_over > 0)
    blockers.push(`${data.lobbies_not_over} eleme lobisi henüz bitmedi; sıralama kesinleşmedi.`);
  if (data && teams.length < cut)
    blockers.push(
      `${FORMAT_LABELS[format]} için ${cut} takım gerekiyor, ${teams.length} takım var.`,
    );

  function build() {
    const message =
      `${FORMAT_LABELS[format]} double elimination bracket'i kurulsun mu?\n\n` +
      `İlk ${cut} takım seed sırasıyla eşleşir (1-${cut}, 2-${cut - 1}…), ${bracketTotal(format)} maç oluşturulur.` +
      (out > 0 ? `\n${out} takım bracket'e giremez ve turnuvadan elenir.` : "");
    if (!window.confirm(message)) return;
    void run(() => makeBracket(format, ids, stageIds));
  }

  function takeDown() {
    if (!data) return;
    if (
      !window.confirm(
        `Bracket'teki ${data.matches} maçın hepsi silinsin mi? Seed sırası olduğu gibi kalır.`,
      )
    )
      return;
    void run(removeBracket);
  }

  return (
    <section className="seeding-page">
      <div className="page-header">
        <h1>Sıralama sonuçları</h1>
        {data && <span className="badge">{ranked}</span>}
        {data && (
          <div className="page-header-aside">
            {data.custom && (
              <span className="seeding-custom" title="Seed sırası eleme sıralamasından farklı">
                <Icon name="pencil" size={14} />
                Elle düzenlendi
              </span>
            )}
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy || locked || !data.custom}
              title={locked ? "Bracket kurulduktan sonra seed sırası değişmez" : undefined}
              onClick={() => run(resetSeeding)}
            >
              <Icon name="undo" size={14} />
              Eleme sırasına döndür
            </button>
          </div>
        )}
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!data && !error && <p className="status">Yükleniyor…</p>}

      {data && (
        <div className="seeding-layout">
          <aside className="seeding-builder" aria-label="Bracket">
            {locked ? (
              <>
                <h2>Bracket kuruldu</h2>
                <p className="seeding-built">
                  <span className="num">{data.matches}</span> maç
                  {data.played > 0 && (
                    <>
                      {" · "}
                      <span className="num">{data.played}</span> oynandı
                    </>
                  )}
                </p>
                <p className="seeding-note">
                  Seed sırası bracket'in kaydı olarak kilitlendi. Yeni bracket için önce bütün
                  maçları kaldır.
                </p>
                <div className="seeding-actions">
                  <Link to="/admin/bracket" className="btn btn-secondary">
                    <Icon name="bracket" size={16} />
                    Maçlara git
                  </Link>
                  <button
                    type="button"
                    className="btn btn-danger"
                    disabled={busy || data.played > 0}
                    onClick={takeDown}
                  >
                    <Icon name="trash" size={16} />
                    Bütün maçları kaldır
                  </button>
                </div>
                {data.played > 0 && (
                  <p className="seeding-blocker">
                    Oynanmış ya da odası açılmış maç var; bracket artık kaldırılamaz.
                  </p>
                )}
              </>
            ) : (
              <>
                <h2>Bracket kur</h2>
                <div className="seeding-formats" role="radiogroup" aria-label="Format">
                  {FORMATS.map((f) => (
                    <label key={f} className="seeding-format">
                      <input
                        type="radio"
                        name="seeding-format"
                        value={f}
                        checked={format === f}
                        onChange={() => setFormat(f)}
                      />
                      <span className="seeding-format-name">{FORMAT_LABELS[f]}</span>
                      <span className="seeding-format-facts">
                        <span className="num">{FORMAT_TEAMS[f]}</span> takım ·{" "}
                        <span className="num">{bracketTotal(f)}</span> maç
                      </span>
                    </label>
                  ))}
                </div>
                <p className="seeding-summary">
                  İlk <strong className="num">{cut}</strong> takım double elimination'a girer
                  {out > 0 ? (
                    <>
                      , <strong className="num seeding-out-count">{out}</strong> takım elenir.
                    </>
                  ) : (
                    "."
                  )}
                </p>

                <ol className="seeding-weeks" aria-label="Haftalar ve stage'leri">
                  {weeks.map((w, i) => (
                    <li key={w.usual} className="seeding-week">
                      <span className="seeding-week-index num">{i + 1}</span>
                      <span className="seeding-week-rounds">
                        {w.rounds}
                        <span className="seeding-week-count">
                          <span className="num">{w.matches}</span> maç
                        </span>
                      </span>
                      <select
                        className="select select-sm"
                        aria-label={`${i + 1}. hafta (${w.rounds}) stage'i`}
                        value={stageIds[i] ?? ""}
                        onChange={(e) => {
                          const next = [...stageIds];
                          next[i] = e.target.value || null;
                          setPicked((p) => ({ ...p, [format]: next }));
                        }}
                      >
                        <option value="">— Stage yok —</option>
                        {data.stages.map((s) => (
                          <option
                            key={s.id}
                            value={s.id}
                            disabled={stageIds.includes(s.id) && stageIds[i] !== s.id}
                          >
                            {s.name}
                          </option>
                        ))}
                      </select>
                    </li>
                  ))}
                </ol>

                {blockers.map((b) => (
                  <p key={b} className="seeding-blocker">
                    {b}
                  </p>
                ))}
                <button
                  type="button"
                  className="btn btn-primary btn-lg seeding-build"
                  disabled={busy || blockers.length > 0}
                  onClick={build}
                >
                  {FORMAT_LABELS[format]} bracket'ini kur
                </button>
                <p className="seeding-note">
                  Maçlar zamansız oluşur; saatleri Maçlar sekmesinden verilir.
                </p>
              </>
            )}
          </aside>

          <div className="seeding-sheet">
            {teams.length === 0 ? (
              <p className="status">Henüz takım yok.</p>
            ) : (
              <ol
                className={locked ? "seeding-list is-locked" : "seeding-list"}
                aria-label="Seed sırası"
              >
                <li className="seeding-head" aria-hidden="true">
                  <span />
                  <span>Seed</span>
                  <span>Takım</span>
                  <span className="is-right">Eleme</span>
                  <span className="is-right seeding-opt">Z toplamı</span>
                  <span className="is-right seeding-opt2">Mapler</span>
                  <span />
                </li>
                {teams.map((team, i) => {
                  // Before the bracket, the cut follows the chosen format; after, who's in it.
                  const isOut = locked ? !team.in_bracket : i >= cut;
                  const showCut = locked ? false : i === cut && teams.length > cut;
                  return (
                    <SeedRowWithCut
                      key={team.id}
                      team={team}
                      seed={i + 1}
                      out={isOut}
                      locked={locked}
                      busy={busy}
                      last={i === teams.length - 1}
                      cutBefore={showCut ? { cut, out } : null}
                      poolSize={data.pool_size}
                      dragProps={locked ? undefined : dragProps(team.id)}
                      onMove={(delta) => applyOrder(moveBy(ids, team.id, delta))}
                      onSeed={(value) => moveToSeed(team, value)}
                    />
                  );
                })}
              </ol>
            )}
          </div>
        </div>
      )}
    </section>
  );
}

const bracketTotal = (format: BracketFormat) =>
  bracketWeeks(format).reduce((n, w) => n + w.matches, 0);

interface RowProps {
  team: SeedTeam;
  seed: number;
  out: boolean;
  locked: boolean;
  busy: boolean;
  last: boolean;
  cutBefore: { cut: number; out: number } | null;
  poolSize: number;
  dragProps?: ReturnType<ReturnType<typeof useDragReorder>>;
  onMove: (delta: number) => void;
  onSeed: (value: string) => void;
}

function SeedRowWithCut({ cutBefore, ...props }: RowProps) {
  return (
    <>
      {cutBefore && (
        <li
          className="seeding-cut"
          aria-label={`Kesim: ilk ${cutBefore.cut} takım bracket'e girer, ${cutBefore.out} takım elenir`}
        >
          <span className="seeding-cut-label">
            Kesim <span className="num">{cutBefore.cut}</span>
          </span>
          <span className="seeding-cut-note">
            Aşağıdaki <span className="num">{cutBefore.out}</span> takım elenir
          </span>
        </li>
      )}
      <SeedRow {...props} />
    </>
  );
}

function SeedRow({
  team,
  seed,
  out,
  locked,
  busy,
  last,
  poolSize,
  dragProps,
  onMove,
  onSeed,
}: Omit<RowProps, "cutBefore">) {
  const q = team.qualifier;
  // How far the hosts moved it from its seed in the qualifier's order (positive: up).
  const moved = team.qualifier_seed - seed;
  const [draft, setDraft] = useState<string | null>(null);

  const commit = () => {
    if (draft !== null && draft.trim() !== "") onSeed(draft);
    setDraft(null);
  };
  const onKey = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "Enter") e.currentTarget.blur();
    if (e.key === "Escape") {
      setDraft(null);
      e.currentTarget.blur();
    }
  };

  const flags: string[] = [];
  if (team.banned) flags.push("Banlanmış oyuncu: eleme sırasında sona alındı");
  if (team.rank_warning) flags.push(RANK_WARNING_LABELS[team.rank_warning]);

  const classes = ["seeding-row", out && "is-out", team.banned && "is-flagged"]
    .filter(Boolean)
    .join(" ");

  return (
    <li className={classes} {...dragProps}>
      <span className="seeding-grip" aria-hidden="true">
        {!locked && <Icon name="grip" size={16} />}
      </span>
      <span className="seeding-seed">
        {locked ? (
          <span className="num">{seed}</span>
        ) : (
          <input
            className="seeding-seed-input num"
            type="number"
            inputMode="numeric"
            min={1}
            aria-label={`${team.name}: seed (yazıp Enter ile taşı)`}
            title="Seed numarası yazıp Enter'a bas: takım oraya taşınır"
            value={draft ?? String(seed)}
            disabled={busy}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={onKey}
          />
        )}
      </span>
      <span className="seeding-team">
        <span className="seeding-team-name">{team.name}</span>
        <span className="seeding-team-players">
          {team.players.map((p) => p.username).join(" · ")}
        </span>
        {flags.length > 0 && <span className="seeding-flag">{flags.join(" · ")}</span>}
      </span>
      <span className="seeding-rank is-right">
        {q ? (
          <>
            <span className="seeding-rank-line">
              {moved !== 0 && !locked && (
                <span
                  className={moved > 0 ? "seeding-moved is-up" : "seeding-moved is-down"}
                  title={`Eleme sırasındaki yerinden ${Math.abs(moved)} sıra ${moved > 0 ? "yukarı" : "aşağı"} taşındı`}
                >
                  <Icon name={moved > 0 ? "arrowUp" : "arrowDown"} size={12} />
                  <span className="num">{Math.abs(moved)}</span>
                </span>
              )}
              <span className="num seeding-rank-figure">#{q.rank}</span>
            </span>
            <span className="seeding-rank-lobby">
              {q.pending ? "lobisi sürüyor" : team.lobby ? `Lobi ${team.lobby}` : ""}
            </span>
          </>
        ) : (
          <span className="seeding-none">Elemelere katılmadı</span>
        )}
      </span>
      <span className="is-right seeding-opt">
        {q ? <ZValue z={q.z_sum} size="lg" /> : <span className="seeding-none">—</span>}
      </span>
      <span className="is-right seeding-opt2 seeding-maps">
        {q ? (
          <>
            <span className="num">
              {q.plays}/{poolSize}
            </span>
            {q.missing > 0 ? (
              <span className="seeding-missing">{q.missing} eksik · 0 sayıldı</span>
            ) : (
              <span className="seeding-total num">{score.format(q.total_score)}</span>
            )}
          </>
        ) : (
          <span className="seeding-none">—</span>
        )}
      </span>
      <span className="seeding-tools">
        {locked ? (
          <span className={team.in_bracket ? "seeding-state is-in" : "seeding-state is-out"}>
            {team.in_bracket ? "Bracket'te" : "Elendi"}
          </span>
        ) : (
          <>
            <IconButton
              icon="arrowUp"
              label={`${team.name} takımını bir sıra yukarı taşı`}
              disabled={busy || seed === 1}
              onClick={() => onMove(-1)}
            />
            <IconButton
              icon="arrowDown"
              label={`${team.name} takımını bir sıra aşağı taşı`}
              disabled={busy || last}
              onClick={() => onMove(1)}
            />
          </>
        )}
      </span>
    </li>
  );
}
