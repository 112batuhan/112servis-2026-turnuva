import { formatDateTime } from "../lib/format";

export function RanksUpdated({ at }: { at: string | null }) {
  return <span className="muted">{at ? `Ranks updated ${formatDateTime(at)}` : "Ranks not updated yet"}</span>;
}
