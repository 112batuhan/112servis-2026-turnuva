import { del, get, patch, post, put } from "./client";
import type {
  Category,
  CategoryKind,
  MapNoteField,
  PoolMap,
  PublicStage,
  PublicStageDetail,
  Slot,
  Stage,
  StageDetail,
} from "./types";

/** A category as the editor creates it. */
export interface NewCategory {
  name: string;
  abbreviation: string;
  kind: CategoryKind;
}

// ---- stages ----
export const fetchStages = () => get<Stage[]>("/api/stages");
export const createStage = (name: string) => post<Stage>("/api/stages", { name });
export const renameStage = (id: string, name: string) =>
  patch<Stage>(`/api/stages/${id}`, { name });
export const reorderStages = (ids: string[]) => post<void>("/api/stages/reorder", { ids });
export const deleteStage = (id: string) => del(`/api/stages/${id}`);
/** null leaves no stage as the qualifier pool. */
export const setQualifierStage = (stageId: string | null) =>
  put<void>("/api/stages/qualifier", { stage_id: stageId });
export const setStageBestOf = (stageId: string, bestOf: number | null) =>
  put<void>(`/api/stages/${stageId}/best-of`, { best_of: bestOf });
/** null (or empty) goes back to the default order. */
/** null (or empty) takes the short name off. */
export const setStageAbbreviation = (stageId: string, abbreviation: string | null) =>
  put<void>(`/api/stages/${stageId}/abbreviation`, { abbreviation });
export const setStageDraftOrder = (stageId: string, order: string | null) =>
  put<void>(`/api/stages/${stageId}/draft-order`, { draft_order: order });

// ---- a stage's pool (everything under /api/mappool) ----
export const fetchStage = (id: string) => get<StageDetail>(`/api/mappool/stages/${id}`);
export const setStagePublished = (id: string, published: boolean) =>
  put<Stage>(`/api/mappool/stages/${id}/published`, { published });
/** null (or empty) takes the pack link off. */
export const setStagePackUrl = (stageId: string, url: string | null) =>
  put<void>(`/api/mappool/stages/${stageId}/pack`, { pack_url: url });

// ---- categories ----
export const createCategory = (stageId: string, category: NewCategory) =>
  post<Category>(`/api/mappool/stages/${stageId}/categories`, category);
export const renameCategory = (id: string, name: string) =>
  patch<Category>(`/api/mappool/categories/${id}`, { name });
export const setCategoryAbbreviation = (id: string, abbreviation: string) =>
  patch<Category>(`/api/mappool/categories/${id}`, { abbreviation });
export const setCategoryKind = (id: string, kind: CategoryKind) =>
  patch<Category>(`/api/mappool/categories/${id}`, { kind });
export const setCategoryColor = (id: string, color: string) =>
  patch<Category>(`/api/mappool/categories/${id}`, { color });
export const reorderCategories = (ids: string[]) =>
  post<void>("/api/mappool/categories/reorder", { ids });
export const deleteCategory = (id: string) => del(`/api/mappool/categories/${id}`);

// ---- slots ----
export const addSlot = (categoryId: string) =>
  post<Slot>(`/api/mappool/categories/${categoryId}/slots`);
export const updateSlotNotes = (slotId: string, editorNotes: string) =>
  patch<Slot>(`/api/mappool/slots/${slotId}`, { editor_notes: editorNotes });
export const deleteSlot = (slotId: string) => del(`/api/mappool/slots/${slotId}`);

// ---- maps ----
// New maps land in the generic pool with their mod-adjusted stats locked in.
export const addMap = (beatmapId: number, mods: string) =>
  post<PoolMap>("/api/mappool/maps", { beatmap_id: beatmapId, mods });
/** slotId null sends the map back to the generic pool. A map already in the target slot is bumped there too. */
export const moveMap = (mapId: string, slotId: string | null) =>
  patch<PoolMap>(`/api/mappool/maps/${mapId}`, { slot_id: slotId });
export const updateMapNote = (mapId: string, field: MapNoteField, value: string) =>
  patch<PoolMap>(`/api/mappool/maps/${mapId}/notes`, { [field]: value });
/** Resolves the map's stats from osu! again (HR stats included for a nomod map). */
export const recomputeMapStats = (mapId: string) =>
  post<PoolMap>(`/api/mappool/maps/${mapId}/stats`);
export const deleteMap = (mapId: string) => del(`/api/mappool/maps/${mapId}`);

// ---- public (published stages only, no auth) ----
export const fetchPublicStages = () => get<PublicStage[]>("/api/public/stages");
export const fetchPublicStage = (id: string) =>
  get<PublicStageDetail>(`/api/public/mappool/stages/${id}`);
