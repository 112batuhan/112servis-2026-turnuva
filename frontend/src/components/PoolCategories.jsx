import MapCard from "./MapCard.jsx";

// Read-only display of a stage's categories and their maps. Shared by the public map
// pool page; mirrors what the editor shows, minus the drag-and-drop. Each map carries
// its own locked mods (shown on the card).
export default function PoolCategories({ categories, maps }) {
  if (categories.length === 0) {
    return <p className="status">No categories in this stage yet.</p>;
  }

  return (
    <div className="pool-board">
      {categories.map((c) => {
        const items = maps.filter((m) => m.category_id === c.id);
        return (
          <section key={c.id} className="pool-section">
            <div className="pool-section-head">
              <span className="pool-section-title">{c.name}</span>
            </div>
            <div className="map-list">
              {items.map((m) => (
                <MapCard key={m.id} bm={m} />
              ))}
              {items.length === 0 && <p className="muted small">No maps.</p>}
            </div>
          </section>
        );
      })}
    </div>
  );
}
