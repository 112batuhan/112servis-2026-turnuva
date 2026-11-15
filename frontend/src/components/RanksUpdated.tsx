import { formatDateTime } from "../lib/format";

export function RanksUpdated({ at }: { at: string | null }) {
  return (
    <span className="muted">
      {at ? `Rank'lar güncellendi: ${formatDateTime(at)}` : "Rank'lar henüz güncellenmedi"}
    </span>
  );
}
