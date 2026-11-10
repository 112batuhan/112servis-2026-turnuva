import { useCallback, useEffect, useState } from "react";
import { errorMessage } from "../../../api/client";
import { useAction } from "../../../lib/useAction";
import * as api from "../../../api/mappool";
import type { MapNoteField, Stage, StageDetail } from "../../../api/types";
import { readString, writeString } from "../../../lib/storage";
import { moveBy, moveToTarget, sortByIds } from "../../../lib/reorder";

const lastStageKey = (userId: string) => `mappool-stage:${userId}`;

/** Makes the map pool editor open on this stage next time. */
export function rememberStage(userId: string, stageId: string) {
  writeString(lastStageKey(userId), stageId);
}

/**
 * All data and actions for the map pool editor. Hosts create, rename, reorder and
 * delete stages on their own page; here they're only picked, published and filled.
 *
 * Structural changes (create/delete/move) go through `run`, which shows the busy state
 * and reloads the stage afterwards. Small edits (notes, colours, ordering) update local
 * state immediately and save in the background.
 */
export function useStageEditor(userId: string) {
  const [stages, setStages] = useState<Stage[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<StageDetail | null>(null);
  const { busy, error, setError, run } = useAction();

  const loadStages = useCallback(async () => {
    const list = await api.fetchStages();
    setStages(list);
    setSelectedId((current) => {
      if (current && list.some((s) => s.id === current)) return current;
      const last = readString(lastStageKey(userId));
      if (last && list.some((s) => s.id === last)) return last;
      return list[0]?.id ?? null;
    });
  }, [userId]);

  useEffect(() => {
    loadStages().catch((e) => setError(errorMessage(e)));
  }, [loadStages]);

  useEffect(() => {
    if (selectedId) rememberStage(userId, selectedId);
  }, [selectedId, userId]);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    let active = true;
    api
      .fetchStage(selectedId)
      .then((d) => active && setDetail(d))
      .catch((e) => active && setError(errorMessage(e)));
    return () => {
      active = false;
    };
  }, [selectedId]);

  async function reloadDetail() {
    if (selectedId) setDetail(await api.fetchStage(selectedId));
  }

  function saveInBackground(request: Promise<unknown>) {
    request.catch((e) => setError(errorMessage(e)));
  }

  function patchDetail(update: (d: StageDetail) => StageDetail) {
    setDetail((d) => (d ? update(d) : d));
  }

  // ---- stages ----

  const togglePublished = () =>
    run(async () => {
      if (!detail) return;
      await api.setStagePublished(detail.id, !detail.published);
      await Promise.all([reloadDetail(), loadStages()]);
    });

  const savePackUrl = (url: string) =>
    run(async () => {
      if (!detail) return;
      await api.setStagePackUrl(detail.id, url.trim() || null);
      await Promise.all([reloadDetail(), loadStages()]);
    });

  // ---- categories ----

  const createCategory = (name: string) =>
    run(async () => {
      if (!selectedId) return;
      await api.createCategory(selectedId, name);
      await reloadDetail();
    });

  const renameCategory = (id: string, name: string) =>
    run(async () => {
      await api.renameCategory(id, name);
      await reloadDetail();
    });

  const deleteCategory = (id: string) =>
    run(async () => {
      await api.deleteCategory(id);
      await reloadDetail();
    });

  /** Live preview while the colour picker moves; call saveCategoryColor when it closes. */
  const previewCategoryColor = (id: string, color: string) =>
    patchDetail((d) => ({ ...d, categories: d.categories.map((c) => (c.id === id ? { ...c, color } : c)) }));

  const saveCategoryColor = (id: string, color: string) => saveInBackground(api.setCategoryColor(id, color));

  function applyCategoryOrder(ids: string[] | null) {
    if (!ids) return;
    patchDetail((d) => ({ ...d, categories: sortByIds(d.categories, ids) }));
    saveInBackground(api.reorderCategories(ids));
  }

  const categoryIds = () => (detail?.categories ?? []).map((c) => c.id);

  const dragCategory = (draggedId: string, targetId: string) =>
    applyCategoryOrder(moveToTarget(categoryIds(), draggedId, targetId));

  const shiftCategory = (id: string, delta: number) => applyCategoryOrder(moveBy(categoryIds(), id, delta));

  // ---- slots ----

  const addSlot = (categoryId: string) =>
    run(async () => {
      await api.addSlot(categoryId);
      await reloadDetail();
    });

  const deleteSlot = (slotId: string) =>
    run(async () => {
      await api.deleteSlot(slotId);
      await reloadDetail();
    });

  function saveSlotNotes(slotId: string, notes: string) {
    patchDetail((d) => ({ ...d, slots: d.slots.map((s) => (s.id === slotId ? { ...s, editor_notes: notes } : s)) }));
    saveInBackground(api.updateSlotNotes(slotId, notes));
  }

  // ---- maps ----

  const addMap = (beatmapId: number, mods: string) =>
    run(async () => {
      await api.addMap(beatmapId, mods);
      await reloadDetail();
    });

  const moveMap = (mapId: string, slotId: string | null) =>
    run(async () => {
      await api.moveMap(mapId, slotId);
      await reloadDetail();
    });

  const recomputeMapStats = (mapId: string) =>
    run(async () => {
      await api.recomputeMapStats(mapId);
      await reloadDetail();
    });

  const deleteMap = (mapId: string) =>
    run(async () => {
      await api.deleteMap(mapId);
      await reloadDetail();
    });

  function saveMapNote(mapId: string, field: MapNoteField, value: string) {
    patchDetail((d) => ({ ...d, maps: d.maps.map((m) => (m.id === mapId ? { ...m, [field]: value } : m)) }));
    saveInBackground(api.updateMapNote(mapId, field, value));
  }

  return {
    stages,
    selectedId,
    selectStage: setSelectedId,
    detail,
    error,
    clearError: () => setError(null),
    busy,
    actions: {
      togglePublished,
      savePackUrl,
      createCategory,
      renameCategory,
      deleteCategory,
      previewCategoryColor,
      saveCategoryColor,
      dragCategory,
      shiftCategory,
      addSlot,
      deleteSlot,
      saveSlotNotes,
      addMap,
      moveMap,
      recomputeMapStats,
      deleteMap,
      saveMapNote,
    },
  };
}

export type StageEditorActions = ReturnType<typeof useStageEditor>["actions"];
