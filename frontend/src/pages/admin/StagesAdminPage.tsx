import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { fetchBracket } from "../../api/bracket";
import { errorMessage } from "../../api/client";
import {
  createStage,
  deleteStage,
  fetchStages,
  renameStage,
  reorderStages,
  setQualifierStage,
} from "../../api/mappool";
import type { Stage } from "../../api/types";
import { useSignedInUser } from "../../auth/AuthContext";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { IconButton } from "../../components/IconButton";
import { InlineEdit } from "../../components/InlineEdit";
import { useDragReorder } from "../../lib/dragAndDrop";
import { moveBy, moveToTarget, sortByIds } from "../../lib/reorder";
import { rememberStage } from "./mappool/useStageEditor";
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
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const [list, bracket] = await Promise.all([fetchStages(), fetchBracket()]);
    const counts = new Map<string, number>();
    for (const m of bracket.matches) {
      if (m.stage) counts.set(m.stage.id, (counts.get(m.stage.id) ?? 0) + 1);
    }
    setStages(list);
    setMatchCounts(counts);
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
        <Link to="/admin/bracket">Bracket</Link> sekmesindeki maç panelinden ayarla.
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
