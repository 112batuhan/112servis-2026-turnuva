import type { PublicPoolMap, RefereeRoom } from "../../../api/types";
import { isFreemodCategory } from "../../../lib/bancho";
import { formatLength } from "../../../lib/format";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./PoolPanel.css";

interface PoolEntry {
  /** "NM1", "HD2", "TB". */
  label: string;
  color: string;
  freemod: boolean;
  map: PublicPoolMap;
}

// The pool in category and slot order. A category with one slot is just its name.
function poolEntries(room: RefereeRoom): PoolEntry[] {
  const entries: PoolEntry[] = [];
  for (const category of room.categories) {
    const slots = room.slots.filter((s) => s.category_id === category.id).sort((a, b) => a.position - b.position);
    slots.forEach((slot, i) => {
      const map = room.maps.find((m) => m.slot_id === slot.id);
      if (!map) return;
      entries.push({
        label: slots.length > 1 ? `${category.name}${i + 1}` : category.name,
        color: category.color,
        freemod: isFreemodCategory(category.name),
        map,
      });
    });
  }
  return entries;
}

/** The match's map pool; picking a map sets it and its mods in the room. */
export function PoolPanel({ referee }: { referee: RefereeRoomState }) {
  const { room, lobby, inChannel, actions } = referee;
  if (!room) return null;
  const entries = poolEntries(room);
  const played = new Set(lobby.results.map((r) => r.beatmapId));

  return (
    <section className="pool-panel" aria-label="Mappool">
      <h2 className="pool-panel-title">
        Mappool
        {room.stage && <span className="muted"> · {room.stage.name}</span>}
      </h2>
      {!room.stage && <p className="muted">Bu maça stage seçilmemiş.</p>}
      {room.stage && entries.length === 0 && <p className="muted">Bu stage'in havuzunda map yok.</p>}
      <ul className="pool-panel-list">
        {entries.map(({ label, color, freemod, map }) => {
          const current = lobby.beatmapId === map.beatmap_id;
          const mods = freemod ? "Freemod" : map.mods || "NM";
          return (
            <li key={map.id}>
              <button
                type="button"
                className={`pool-pick ${current ? "is-current" : ""} ${played.has(map.beatmap_id) ? "is-played" : ""}`}
                disabled={!inChannel}
                title={inChannel ? `${label} seç: !mp map ${map.beatmap_id}, ${mods}` : "Önce odanın kanalına katıl"}
                onClick={() => actions.pickMap(map, freemod)}
              >
                <span className="pool-pick-label" style={{ borderColor: color, color }}>
                  {label}
                </span>
                <span className="pool-pick-text">
                  <span className="pool-pick-title">
                    {map.artist} - {map.title}
                  </span>
                  <span className="pool-pick-meta">
                    [{map.version}] · {map.star_rating.toFixed(2)}★ · {Math.round(map.bpm)} BPM ·{" "}
                    {formatLength(map.total_length)} · {mods}
                  </span>
                </span>
                {current && <span className="badge badge-info">Seçili</span>}
                {!current && played.has(map.beatmap_id) && <span className="badge">Oynandı</span>}
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}
