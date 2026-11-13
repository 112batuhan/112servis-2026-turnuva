import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { fetchBracket } from "../../api/bracket";
import { fetchPickem, setStagePickemMode } from "../../api/pickem";
import { errorMessage } from "../../api/client";
import { useAction } from "../../lib/useAction";
import {
  createStage,
  deleteStage,
  fetchStages,
  renameStage,
  reorderStages,
  setQualifierStage,
  setStageBestOf,
  setStageDraftOrder,
} from "../../api/mappool";
import type { PickemMode, PickemStage, Stage } from "../../api/types";
import { useSignedInUser } from "../../auth/AuthContext";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { IconButton } from "../../components/IconButton";
import { InlineEdit } from "../../components/InlineEdit";
import { BEST_OF_CHOICES, bestOfLabel, firstTo } from "../../lib/bestOf";
import {
  DEFAULT_DRAFT_ORDER,
  EXAMPLE_DRAFT_ORDER,
  formatDraftOrder,
  parseDraftOrder,
  stepLabel,
  type DraftOrder,
} from "../../lib/draftOrder";
import { STATUS_LABELS } from "../../lib/pickem";
import { useDragReorder } from "../../lib/dragAndDrop";
import { moveBy, moveToTarget, sortByIds } from "../../lib/reorder";
import { readString, removeString, writeString } from "../../lib/storage";
import { rememberStage } from "./mappool/useStageEditor";
import "../../components/bracket/BracketViewTabs.css";
import "./StagesAdminPage.css";

/**
 * Host-only. Stages are the tournament's rounds: each has a map pool (filled on the
 * Mappool tab) and bracket matches are played in one (picked in the match panel).
 */
export function StagesAdminPage() {
  const user = useSignedInUser();
  const [stages, setStages] = useState<Stage[] | null>(null);
  // Bracket matches per stage id.
  const [matchCounts, setMatchCounts] = useState(new Map<string, number>());
  // Each stage's pick'em as it stands, for the stages that have one.
  const [pickem, setPickem] = useState(new Map<string, PickemStage>());
  // A change, then a reload either way.
  const { busy, error, setError, run } = useAction(() => load());

  const load = useCallback(async () => {
    const [list, bracket, picks] = await Promise.all([fetchStages(), fetchBracket(), fetchPickem()]);
    const counts = new Map<string, number>();
    for (const m of bracket.matches) {
      if (m.stage) counts.set(m.stage.id, (counts.get(m.stage.id) ?? 0) + 1);
    }
    setStages(list);
    setMatchCounts(counts);
    setPickem(new Map(picks.stages.map((s) => [s.id, s])));
  }, []);

  useEffect(() => {
    load().catch((e) => setError(errorMessage(e)));
  }, [load, setError]);

  const ids = useMemo(() => (stages ?? []).map((s) => s.id), [stages]);
  const reorderProps = useDragReorder((dragged, target) => applyOrder(moveToTarget(ids, dragged, target)));

  // Ordering applies right away and saves in the background.
  function applyOrder(next: string[] | null) {
    if (!next) return;
    setStages((list) => list && sortByIds(list, next));
    reorderStages(next).catch((e) => {
      setError(errorMessage(e));
      load().catch(() => {});
    });
  }

  // Applies right away and saves in the background, like the order.
  function applyBestOf(stage: Stage, bestOf: number | null) {
    setStages((list) => list && list.map((s) => (s.id === stage.id ? { ...s, best_of: bestOf } : s)));
    setStageBestOf(stage.id, bestOf).catch((e) => {
      setError(errorMessage(e));
      load().catch(() => {});
    });
  }

  // Applies right away and saves in the background; null goes back to the default order.
  function applyDraftOrder(stage: Stage, order: string | null) {
    setStages((list) => list && list.map((s) => (s.id === stage.id ? { ...s, draft_order: order } : s)));
    setStageDraftOrder(stage.id, order).catch((e) => {
      setError(errorMessage(e));
      load().catch(() => {});
    });
  }

  // Saves, then reloads to show what the stage's pick'em is now.
  function applyPickemMode(stage: Stage, mode: PickemMode) {
    setPickem((m) => {
      const current = m.get(stage.id);
      return current ? new Map(m).set(stage.id, { ...current, mode }) : m;
    });
    setStagePickemMode(stage.id, mode)
      .catch((e) => setError(errorMessage(e)))
      .finally(() => load().catch(() => {}));
  }

  function remove(stage: Stage) {
    const count = matchCounts.get(stage.id) ?? 0;
    const matches = count > 0 ? ` ${count} bracket maçı stage'siz kalır.` : "";
    if (
      confirm(
        `"${stage.name}" stage'i silinsin mi? Kategorileri ve slot'ları kaldırılır; map'ler ortak havuza döner.${matches}`,
      )
    ) {
      run(() => deleteStage(stage.id));
    }
  }

  return (
    <section className="panel stages-admin">
      <div className="page-header">
        <h1>Stage'ler</h1>
        {stages && <span className="badge">{stages.length} stage</span>}
      </div>
      <StagesHelp />
      <ErrorAlert message={error} onDismiss={() => setError(null)} />

      <NewStageForm busy={busy} onCreate={(name) => run(() => createStage(name))} />

      {stages && stages.length > 0 && (
        <label className="stages-admin-qualifier">
          <span>Eleme mappool'u</span>
          <select
            className="select"
            value={stages.find((s) => s.is_qualifier)?.id ?? ""}
            disabled={busy}
            onChange={(e) => run(() => setQualifierStage(e.target.value || null))}
          >
            <option value="">— Seçilmedi —</option>
            {stages.map((s) => (
              <option key={s.id} value={s.id}>
                {s.name}
              </option>
            ))}
          </select>
          <span className="stages-admin-qualifier-hint">Eleme lobileri bu stage'in map'leriyle oynanır.</span>
        </label>
      )}

      {stages === null && !error && <p className="status">Yükleniyor…</p>}
      {stages?.length === 0 && <p className="status">Henüz stage yok.</p>}

      {stages && stages.length > 0 && (
        <ol className="stage-list">
          {stages.map((stage, i) => {
            const count = matchCounts.get(stage.id) ?? 0;
            return (
              <li key={stage.id} className="stage-row" {...reorderProps(stage.id)}>
                <span className="stage-row-grip" title="Sıralamak için sürükle">
                  <Icon name="grip" />
                </span>
                <span className="stage-row-name">
                  <InlineEdit
                    value={stage.name}
                    label={`${stage.name} stage'ini yeniden adlandır`}
                    onSave={(name) => run(() => renameStage(stage.id, name))}
                  />
                </span>
                {stage.is_qualifier && <span className="badge badge-info">Eleme</span>}
                <span
                  className={`badge ${stage.published ? "badge-success" : "badge-warning"}`}
                  title={stage.published ? "Mappool herkese açık" : "Mappool henüz yayınlanmadı"}
                >
                  {stage.published ? "Yayında" : "Taslak"}
                </span>
                <select
                  className="select select-sm stage-row-bo"
                  value={stage.best_of ?? ""}
                  aria-label={`${stage.name}: kaç map üzerinden oynanıyor`}
                  title="Maçlar kaç map üzerinden oynanıyor"
                  onChange={(e) => applyBestOf(stage, e.target.value ? Number(e.target.value) : null)}
                >
                  <option value="">Bo —</option>
                  {BEST_OF_CHOICES.map((n) => (
                    <option key={n} value={n}>
                      {bestOfLabel(n)} · {firstTo(n)} alan kazanır
                    </option>
                  ))}
                  {stage.best_of !== null && !(BEST_OF_CHOICES as readonly number[]).includes(stage.best_of) && (
                    <option value={stage.best_of}>{bestOfLabel(stage.best_of)}</option>
                  )}
                </select>
                {!stage.is_qualifier && (
                  <DraftOrderField stage={stage} onSave={(order) => applyDraftOrder(stage, order)} />
                )}
                {pickem.has(stage.id) && (
                  <PickemControl stage={stage} pickem={pickem.get(stage.id)!} onChange={(mode) => applyPickemMode(stage, mode)} />
                )}
                <span className="stage-row-meta">{count} maç</span>
                <Link
                  to="/admin/mappool"
                  className="btn btn-secondary btn-sm"
                  onClick={() => rememberStage(user.id, stage.id)}
                >
                  Mappool
                </Link>
                <IconButton
                  icon="arrowUp"
                  label={`${stage.name} stage'ini yukarı taşı`}
                  disabled={i === 0}
                  onClick={() => applyOrder(moveBy(ids, stage.id, -1))}
                />
                <IconButton
                  icon="arrowDown"
                  label={`${stage.name} stage'ini aşağı taşı`}
                  disabled={i === stages.length - 1}
                  onClick={() => applyOrder(moveBy(ids, stage.id, 1))}
                />
                <IconButton
                  icon="trash"
                  label={`${stage.name} stage'ini sil`}
                  tone="danger"
                  disabled={busy}
                  onClick={() => remove(stage)}
                />
              </li>
            );
          })}
        </ol>
      )}
    </section>
  );
}

/**
 * A stage's pick and ban order as text. While it's being typed a popover under it reads it back
 * (each step's icon and team, then "sırayla"), or says what's wrong with it, which stays until it's
 * fixed. Enter or leaving the field saves it written the stored way; Escape puts back the saved one;
 * emptied, the stage goes back to the default.
 */
function DraftOrderField({ stage, onSave }: { stage: Stage; onSave: (order: string | null) => void }) {
  const saved = stage.draft_order ?? "";
  const [text, setText] = useState(saved);
  const [editing, setEditing] = useState(false);
  useEffect(() => setText(saved), [saved]);
  const parsed = text.trim() ? parseDraftOrder(text) : parseDraftOrder(DEFAULT_DRAFT_ORDER);
  const error = "error" in parsed ? parsed.error : null;
  const errorId = `stage-order-error-${stage.id}`;
  const shown = text || DEFAULT_DRAFT_ORDER;

  function commit() {
    setEditing(false);
    if (!text.trim()) {
      if (saved) onSave(null);
      return;
    }
    if ("order" in parsed) {
      const next = formatDraftOrder(parsed.order);
      setText(next);
      if (next !== saved) onSave(next);
    }
  }

  return (
    <span className="stage-row-order">
      <input
        className={`input input-sm stage-row-order-input${error ? " is-invalid" : ""}`}
        value={text}
        placeholder={DEFAULT_DRAFT_ORDER}
        size={shown.length + 1}
        spellCheck={false}
        autoCapitalize="off"
        autoComplete="off"
        aria-label={`${stage.name}: pick-ban sırası`}
        aria-invalid={error !== null}
        aria-describedby={error ? errorId : undefined}
        title={`Pick-ban sırası, ör. ${EXAMPLE_DRAFT_ORDER}`}
        onChange={(e) => setText(e.target.value)}
        onFocus={() => setEditing(true)}
        onBlur={commit}
        onKeyDown={(e) => {
          if (e.key === "Enter") e.currentTarget.blur();
          if (e.key === "Escape") {
            setText(saved);
            e.currentTarget.blur();
          }
        }}
      />
      {error ? (
        <span className="stage-row-order-pop is-error" id={errorId} role="alert">
          {error}
        </span>
      ) : (
        editing && (
          <span className="stage-row-order-pop">
            <OrderPreview order={(parsed as { order: DraftOrder }).order} fallback={!text.trim()} />
          </span>
        )
      )}
    </span>
  );
}

/** An order read back step by step: a ban or pick icon with its team's figure, then "sırayla". */
function OrderPreview({ order, fallback }: { order: DraftOrder; fallback: boolean }) {
  const bans = order.steps.filter((s) => s.kind === "ban").length;
  const picks = order.steps.length - bans;
  const summary = `${picks} pick, ${bans} ban${order.alternate ? ", sonra sırayla pick" : ""}${fallback ? " (varsayılan)" : ""}`;
  return (
    <>
      <ol className={`stage-order-preview${fallback ? " is-default" : ""}`} aria-label={summary}>
        {order.steps.map((step, i) => (
          <li
            key={i}
            className={`stage-order-step is-${step.kind}`}
            aria-label={`${stepLabel(step.kind)} ${step.team}`}
          >
            <Icon name={step.kind === "ban" ? "ban" : "target"} size={11} />
            {step.team}
          </li>
        ))}
        {order.alternate && (
          <li className="stage-order-step is-alternate" aria-label="sonra sırayla pick">
            <Icon name="swap" size={11} />
          </li>
        )}
      </ol>
      <span className="stage-order-summary" aria-hidden="true">
        {summary}
      </span>
    </>
  );
}

const PICKEM_MODES: { mode: PickemMode; label: string; hint: string }[] = [
  { mode: "auto", label: "Otomatik", hint: "Önceki stage bitince açılır, ilk maçla kilitlenir" },
  { mode: "open", label: "Aç", hint: "Şimdi açık, saati geçmiş olsa bile; her maç odası açılınca ya da bitince kilitlenir" },
  { mode: "closed", label: "Kapat", hint: "Şimdi kilitli; tahminlerin dağılımı açıklanır" },
];

/** A stage's pick'em: what it is now, and the host's override. */
function PickemControl({
  stage,
  pickem,
  onChange,
}: {
  stage: Stage;
  pickem: PickemStage;
  onChange: (mode: PickemMode) => void;
}) {
  return (
    <span className="stage-row-pickem">
      <span className={`stage-row-pickem-status is-${pickem.status}`} title={`Pick'em: ${STATUS_LABELS[pickem.status]}`}>
        {STATUS_LABELS[pickem.status]}
      </span>
      <span className="bracket-view-tabs" role="group" aria-label={`${stage.name} pick'em`}>
        {PICKEM_MODES.map((m) => (
          <button
            key={m.mode}
            type="button"
            className="bracket-view-tab"
            aria-pressed={pickem.mode === m.mode}
            title={m.hint}
            onClick={() => pickem.mode !== m.mode && onChange(m.mode)}
          >
            {m.label}
          </button>
        ))}
      </span>
    </span>
  );
}

const HELP_COLLAPSED_KEY = "stages-admin-help-collapsed";
/** The best of the help's example uses: the bracket's usual length. */
const BEST_OF_EXAMPLE = 9;

/**
 * How the page works, as a ruled sheet: one row per control on a stage's row, in the row's order.
 * Open until a host closes it; that's remembered.
 */
function StagesHelp() {
  const [open, setOpen] = useState(() => readString(HELP_COLLAPSED_KEY) === null);
  const example = parseDraftOrder(EXAMPLE_DRAFT_ORDER);
  return (
    <details
      className="stages-help"
      open={open}
      onToggle={(e) => {
        const next = e.currentTarget.open;
        setOpen(next);
        if (next) removeString(HELP_COLLAPSED_KEY);
        else writeString(HELP_COLLAPSED_KEY, "1");
      }}
    >
      <summary>Nasıl çalışır?</summary>
      <div className="stages-help-sheet">
        <section className="stages-help-row">
          <h2>Stage'ler</h2>
          <div className="stages-help-body">
            <p>
              Turnuvanın turları, oynandıkları sırayla: sürükleyerek ya da oklarla sırala, kalemle yeniden adlandır.
              Kategoriler, map'ler ve yayınlama <Link to="/admin/mappool">Mappool</Link> sekmesinde (yayınlanana kadar
              mappool <em>Taslak</em>); bir maçın stage'i <Link to="/admin/bracket">Bracket</Link> sekmesindeki maç
              panelinde seçilir.
            </p>
          </div>
        </section>

        <section className="stages-help-row">
          <h2>Eleme mappool'u</h2>
          <div className="stages-help-body">
            <p>
              Eleme lobileri seçilen stage'in map'leriyle oynanır. Eleme stage'inin pick-ban sırası ve pick'em'i olmaz.
            </p>
          </div>
        </section>

        <section className="stages-help-row">
          <h2>Bo</h2>
          <div className="stages-help-body">
            <p>
              Maçların kaç map üzerinden oynandığı: <span className="stages-help-code">Bo{BEST_OF_EXAMPLE}</span>'da{" "}
              <span className="num">{firstTo(BEST_OF_EXAMPLE)}</span> map alan kazanır. Hakem sayfası skoru buna göre
              tutar ve beraberlikte tiebreaker'ı açar; maç sayfası ve Discord da gösterir. Bo'su seçilmemiş stage'in
              maçları pick'em'de tahmin edilemez.
            </p>
          </div>
        </section>

        <section className="stages-help-row">
          <h2>Pick-ban sırası</h2>
          <div className="stages-help-body">
            <p>Maçın ban ve pick'leri sırayla, tireyle ayrılmış:</p>
            <dl className="stages-help-list is-codes">
              <div>
                <dt className="stages-help-code">p</dt>
                <dd>pick, oynanan map</dd>
              </div>
              <div>
                <dt className="stages-help-code">b</dt>
                <dd>ban</dd>
              </div>
              <div>
                <dt className="stages-help-code">1</dt>
                <dd>roll'u kazanan: hakem sayfasında ilk pick'i ya da ban'ı işaretlenen takım</dd>
              </div>
              <div>
                <dt className="stages-help-code">2</dt>
                <dd>diğer takım</dd>
              </div>
              <div>
                <dt className="stages-help-code">*</dt>
                <dd>sonda: ardından pick'ler sırayla</dd>
              </div>
            </dl>
            {"order" in example && (
              <p className="stages-help-example">
                <span className="stages-help-code">{EXAMPLE_DRAFT_ORDER}</span>
                <Icon name="arrowRight" size={12} />
                <OrderPreview order={example.order} fallback={false} />
              </p>
            )}
            <p>
              Boş bırakılırsa <span className="stages-help-code">{DEFAULT_DRAFT_ORDER}</span>. Yazarken alanın altında
              böyle okunur; Enter kaydeder, Esc vazgeçer.
            </p>
          </div>
        </section>

        <section className="stages-help-row">
          <h2>Pick'em</h2>
          <div className="stages-help-body">
            <p>Bracket maçı olan, eleme dışındaki stage'lerde. Durumu satırda yazar; düğmeler:</p>
            <dl className="stages-help-list">
              <div>
                <dt>Otomatik</dt>
                <dd>Önceki stage'in bütün maçları bitince açılır, stage'in ilk maçı başlayınca kilitlenir.</dd>
              </div>
              <div>
                <dt>Aç</dt>
                <dd>Şimdi açar, saati geçmiş olsa bile; her maç kendi odası açılınca ya da sonucu girilince kilitlenir.</dd>
              </div>
              <div>
                <dt>Kapat</dt>
                <dd>Hemen kilitler; tahminlerin dağılımı açıklanır.</dd>
              </div>
            </dl>
          </div>
        </section>
      </div>
    </details>
  );
}

function NewStageForm({ busy, onCreate }: { busy: boolean; onCreate: (name: string) => Promise<boolean> }) {
  const [name, setName] = useState("");

  async function submit(e: FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed && (await onCreate(trimmed))) setName("");
  }

  return (
    <form className="inline-form stages-admin-form" onSubmit={submit}>
      <label className="sr-only" htmlFor="new-stage-name">
        Yeni stage adı
      </label>
      <input
        id="new-stage-name"
        className="input"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Stage adı, ör. Round of 32"
      />
      <button type="submit" className="btn btn-primary" disabled={busy || !name.trim()}>
        <Icon name="plus" />
        Stage ekle
      </button>
    </form>
  );
}
