import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { fetchBracket } from "../../api/bracket";
import { fetchPickem, setStagePickemMode } from "../../api/pickem";
import { errorMessage } from "../../api/client";
import {
  createStage,
  deleteStage,
  fetchStages,
  renameStage,
  reorderStages,
  setQualifierStage,
  setStageBestOf,
} from "../../api/mappool";
import type { PickemMode, PickemStage, Stage } from "../../api/types";
import { useSignedInUser } from "../../auth/AuthContext";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { IconButton } from "../../components/IconButton";
import { InlineEdit } from "../../components/InlineEdit";
import { BEST_OF_CHOICES, bestOfLabel, firstTo } from "../../lib/bestOf";
import { STATUS_LABELS } from "../../lib/pickem";
import { useDragReorder } from "../../lib/dragAndDrop";
import { moveBy, moveToTarget, sortByIds } from "../../lib/reorder";
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
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

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
  }, [load]);

  const ids = useMemo(() => (stages ?? []).map((s) => s.id), [stages]);
  const reorderProps = useDragReorder((dragged, target) => applyOrder(moveToTarget(ids, dragged, target)));

  /** Runs a change, then reloads. Resolves true on success so forms know to clear. */
  async function run(change: () => Promise<unknown>): Promise<boolean> {
    setError(null);
    setBusy(true);
    try {
      await change();
      return true;
    } catch (e) {
      setError(errorMessage(e));
      return false;
    } finally {
      await load().catch((e) => setError(errorMessage(e)));
      setBusy(false);
    }
  }

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
      <p className="stages-admin-hint">
        Stage'ler turnuvanın turları. Her birinin bir mappool'u olur ve bracket maçları bir stage'de oynanır. Mappool'un
        kategorilerini ve map'lerini <Link to="/admin/mappool">Mappool</Link> sekmesinde, maçların stage'ini{" "}
        <Link to="/admin/bracket">Bracket</Link> sekmesindeki maç panelinden ayarla. Bo, stage'in maçlarının kaç map
        üzerinden oynandığı: hakem sayfası skoru buna göre takip eder, beraberlikte tiebreaker'ı açar, maç sayfası ve
        Discord da gösterir. Pick'em, bir önceki stage'in bütün maçları bitince açılır ve stage'in ilk maçıyla kilitlenir;
        gerekirse "Aç" ile erken açabilir (ilk maç başlayınca yine kilitlenir) ya da "Kapat" ile hemen kilitleyebilirsin.
      </p>
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

const PICKEM_MODES: { mode: PickemMode; label: string; hint: string }[] = [
  { mode: "auto", label: "Otomatik", hint: "Önceki stage bitince açılır, ilk maçla kilitlenir" },
  { mode: "open", label: "Aç", hint: "Şimdi açık; ilk maç başlayınca yine kilitlenir" },
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
