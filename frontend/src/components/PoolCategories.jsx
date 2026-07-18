import MapCard from "./MapCard.jsx";

// Read-only display of a stage's categories, their slots, and the map in each slot.
// Shared by the public map pool page; mirrors the editor minus the notes and editing.
export default function PoolCategories({ categories, slots = [], maps }) {
  if (categories.length === 0) {
    return <p className="status">No categories in this stage yet.</p>;
  }

  return (
    <div className="pool-board">
      {categories.map((c) => {
        const catSlots = slots.filter((s) => s.category_id === c.id);
        return (
          <section key={c.id} className="pool-section">
            <div className="pool-section-head">
              <span className="pool-section-title">{c.name}</span>
              {catSlots.length > 0 && (
                <span className="muted small">
                  {catSlots.length} slot{catSlots.length === 1 ? "" : "s"}
                </span>
              )}
            </div>
            <div className="slot-grid">
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
