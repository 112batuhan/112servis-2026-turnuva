import MapCard from "./MapCard.jsx";
import { useCollapsed } from "../useCollapsed.js";

// Read-only display of a stage's categories, their slots, and the map in each slot.
// Shared by the public map pool page; mirrors the editor minus the notes and editing.
// Categories collapse on clicking their header; the choice persists per user.
export default function PoolCategories({ categories, slots = [], maps }) {
  const [collapsed, toggleCollapsed] = useCollapsed("public-pool-collapsed");

  if (categories.length === 0) {
    return <p className="status">No categories in this stage yet.</p>;
  }

  return (
    <div className="pool-board">
      {categories.map((c) => {
        const catSlots = slots.filter((s) => s.category_id === c.id);
        const isCollapsed = collapsed.has(c.id);
        return (
          <section key={c.id} className={`pool-section ${isCollapsed ? "is-collapsed" : ""}`}>
            <div
              className="pool-section-head pool-section-head-toggle"
              onClick={() => toggleCollapsed(c.id)}
            >
              <span className="pool-section-title">{c.name}</span>
              <span className="caret" aria-hidden="true">
                ▾
              </span>
            </div>
            <div className="slot-grid" hidden={isCollapsed}>
              {catSlots.length === 0 && <p className="muted small">No slots.</p>}
              {catSlots.map((slot, i) => {
                const slotMap = maps.find((m) => m.slot_id === slot.id);
                return (
                  <div key={slot.id} className="slot">
                    <span className="slot-label">#{i + 1}</span>
                    <div className="slot-body">
                      {slotMap ? (
                        <MapCard bm={slotMap} />
                      ) : (
                        <div className="slot-empty muted small">Empty</div>
                      )}
                    </div>
                  </div>
                );
              })}
            </div>
          </section>
        );
      })}
    </div>
  );
}
