import MapCard from "./MapCard.jsx";

// Read-only display of a stage's categories and their maps, with each category's
// modifier applied to the stats. Shared by the public map pool page; mirrors what
// the editor shows, minus the drag-and-drop.
export default function PoolCategories({ categories, entries }) {
  if (categories.length === 0) {
    return <p className="status">No categories in this stage yet.</p>;
  }

  return (
    <div className="pool-board">
      {categories.map((c) => {
        const items = entries.filter((en) => en.category_id === c.id);
        return (
          <section key={c.id} className="pool-section">
            <div className="pool-section-head">
              <span className="pool-section-title">{c.name}</span>
              {c.modifier && <span className="mod-badge">{c.modifier}</span>}
            </div>
            <div className="map-list">
              {items.map((en) => (
                <MapCard key={en.id} bm={en} modifier={c.modifier} />
              ))}
              {items.length === 0 && <p className="muted small">No maps.</p>}
            </div>
          </section>
        );
      })}
    </div>
  );
}
