import { del, get, patch, post } from "./client";
import type {
  Category,
  MapNoteField,
  PoolMap,
  PublicStageDetail,
  Slot,
  Stage,
  StageDetail,
} from "./types";

// ---- stages ----
export const fetchStages = () => get<Stage[]>("/api/stages");
export const fetchStage = (id: string) => get<StageDetail>(`/api/stages/${id}`);
export const createStage = (name: string) => post<Stage>("/api/stages", { name });
export const renameStage = (id: string, name: string) => patch<Stage>(`/api/stages/${id}`, { name });
export const setStagePublished = (id: string, published: boolean) =>
  patch<Stage>(`/api/stages/${id}`, { published });
export const reorderStages = (ids: string[]) => post<void>("/api/stages/reorder", { ids });
export const deleteStage = (id: string) => del(`/api/stages/${id}`);

// ---- categories ----
export const createCategory = (stageId: string, name: string) =>
  post<Category>(`/api/stages/${stageId}/categories`, { name });
export const renameCategory = (id: string, name: string) =>
  patch<Category>(`/api/categories/${id}`, { name });
export const setCategoryColor = (id: string, color: string) =>
  patch<Category>(`/api/categories/${id}`, { color });
export const reorderCategories = (ids: string[]) => post<void>("/api/categories/reorder", { ids });
export const deleteCategory = (id: string) => del(`/api/categories/${id}`);

// ---- slots ----
export const addSlot = (categoryId: string) => post<Slot>(`/api/categories/${categoryId}/slots`);
export const updateSlotNotes = (slotId: string, editorNotes: string) =>
  patch<Slot>(`/api/slots/${slotId}`, { editor_notes: editorNotes });
export const deleteSlot = (slotId: string) => del(`/api/slots/${slotId}`);

// ---- maps ----
// New maps land in the generic pool with their mod-adjusted stats locked in.
export const addMap = (beatmapId: number, mods: string) =>
  post<PoolMap>("/api/pool", { beatmap_id: beatmapId, mods });
/** slotId null sends the map back to the generic pool. A map already in the target slot is bumped there too. */
export const moveMap = (mapId: string, slotId: string | null) =>
  patch<PoolMap>(`/api/maps/${mapId}`, { slot_id: slotId });
export const updateMapNote = (mapId: string, field: MapNoteField, value: string) =>
  patch<PoolMap>(`/api/maps/${mapId}/notes`, { [field]: value });
export const deleteMap = (mapId: string) => del(`/api/maps/${mapId}`);

// ---- public (published stages only, no auth) ----
export const fetchPublicStages = () => get<Stage[]>("/api/public/stages");
export const fetchPublicStage = (id: string) => get<PublicStageDetail>(`/api/public/stages/${id}`);
