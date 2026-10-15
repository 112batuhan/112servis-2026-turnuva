import { get, put } from "./client";
import type { Rules } from "./types";

export const fetchRules = () => get<Rules>("/api/public/rules");
/** `base` is the `updated_at` the editor loaded; the save fails if someone has saved since. */
export const saveRules = (bbcode: string, base: string) => put<Rules>("/api/rules", { bbcode, base });
