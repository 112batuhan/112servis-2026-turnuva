import { useEffect, useState } from "react";
import {
  fetchStages,
  createStage,
  deleteStage,
  fetchStage,
  setStagePublished,
  createCategory,
  deleteCategory,
  addToGenericPool,
  removeFromGenericPool,
  categorize,
  moveEntry,
  deleteEntry,
} from "../api.js";
import MapCard from "../components/MapCard.jsx";

const MODIFIERS = ["", "HD", "HR", "DT", "HT", "EZ", "FL", "HDHR", "HDDT"];

const allowDrop = (e) => e.preventDefault();

export default function MapPoolPage() {
  const [stages, setStages] = useState([]);
  const [selectedId, setSelectedId] = useState(null);
  const [detail, setDetail] = useState(null);
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);

  const [newStage, setNewStage] = useState("");
  const [mapId, setMapId] = useState("");
  const [catName, setCatName] = useState("");
  const [catMod, setCatMod] = useState("");

  useEffect(() => {
    loadStages();
  }, []);

  async function loadStages() {
    try {
      const list = await fetchStages();
      setStages(list);
      setSelectedId((cur) => cur ?? list[0]?.id ?? null);
    } catch (e) {
      setError(e.message);
    }
  }

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    let active = true;
    fetchStage(selectedId)
      .then((d) => active && setDetail(d))
      .catch((e) => active && setError(e.message));
    return () => {
      active = false;
    };
  }, [selectedId]);

  async function reload() {
    if (selectedId) setDetail(await fetchStage(selectedId));
  }

  async function run(fn) {
    setError(null);
    setBusy(true);
    try {
      await fn();
    } catch (e) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }

  const handleCreateStage = (e) => {
    e.preventDefault();
    const name = newStage.trim();
    if (!name) return;
    run(async () => {
      const s = await createStage(name);
      setNewStage("");
      await loadStages();
      setSelectedId(s.id);
    });
  };

  const handleDeleteStage = () => {
    if (!selectedId || !confirm("Delete this stage? (Maps stay in the generic pool.)")) return;
    run(async () => {
      await deleteStage(selectedId);
      setSelectedId(null);
      await loadStages();
    });
  };

  const handleAddCategory = (e) => {
    e.preventDefault();
    const name = catName.trim();
    if (!name || !selectedId) return;
    run(async () => {
      await createCategory(selectedId, name, catMod || null);
      setCatName("");
      setCatMod("");
      await reload();
    });
  };

  const handleAddMap = (e) => {
    e.preventDefault();
    const id = parseInt(mapId, 10);
    if (!id) return;
    run(async () => {
      await addToGenericPool(id);
      setMapId("");
      await reload();
    });
  };

  const handleDeleteCategory = (id) => run(async () => {
    await deleteCategory(id);
    await reload();
  });

  const handleTogglePublish = () => run(async () => {
    await setStagePublished(selectedId, !detail.published);
    await reload();
    await loadStages();
  });

  // Drop onto a category: a generic map gets categorised; an entry moves category.
  const onDropCategory = (categoryId) => (e) => {
    e.preventDefault();
    const payload = readDrag(e);
    if (!payload) return;
    if (payload.kind === "generic") {
      run(async () => {
        await categorize(selectedId, payload.beatmapId, categoryId);
        await reload();
      });
    } else if (payload.kind === "entry") {
      run(async () => {
        await moveEntry(payload.entryId, categoryId);
        await reload();
      });
    }
  };

  // Drop onto the generic pool: an entry gets uncategorised (returns to the pool).
  const onDropGeneric = (e) => {
    e.preventDefault();
    const payload = readDrag(e);
    if (payload?.kind === "entry") {
      run(async () => {
        await deleteEntry(payload.entryId);
        await reload();
      });
    }
  };

  const categories = detail?.categories ?? [];
  const entries = detail?.entries ?? [];
  const generic = detail?.generic ?? [];

  return (
    <div className="mappool">
      {error && <p className="status status-error">{error}</p>}

      <div className="stage-tabs">
        {stages.map((s) => (
          <button
            key={s.id}
            className={`stage-tab ${s.id === selectedId ? "stage-tab-active" : ""}`}
            onClick={() => setSelectedId(s.id)}
          >
            {s.name}
            {!s.published && <span className="draft-dot" title="Draft — not visible publicly" />}
          </button>
        ))}
        <form className="stage-new" onSubmit={handleCreateStage}>
          <input value={newStage} onChange={(e) => setNewStage(e.target.value)} placeholder="New stage…" />
          <button type="submit" disabled={busy}>
            Add
          </button>
        </form>
      </div>

      {!selectedId && <p className="status">Create a stage to start building a pool.</p>}

      {detail && (
        <>
          <div className="panel mappool-settings">
            <div className="panel-head">
              <h2>
                {detail.name} · settings
                <span className={`stage-status ${detail.published ? "is-published" : ""}`}>
                  {detail.published ? "Published" : "Draft"}
                </span>
              </h2>
              <div className="settings-actions">
                <button
                  className={`publish-btn ${detail.published ? "is-published" : ""}`}
                  onClick={handleTogglePublish}
                  disabled={busy}
                >
                  {detail.published ? "Unpublish" : "Publish"}
                </button>
                <button className="danger-btn" onClick={handleDeleteStage} disabled={busy}>
                  Delete stage
                </button>
              </div>
            </div>
            <form className="cat-form" onSubmit={handleAddCategory}>
              <input
                value={catName}
                onChange={(e) => setCatName(e.target.value)}
                placeholder="Category name (e.g. DoubleTime)"
              />
              <select value={catMod} onChange={(e) => setCatMod(e.target.value)}>
                {MODIFIERS.map((m) => (
                  <option key={m} value={m}>
                    {m === "" ? "No modifier" : m}
                  </option>
                ))}
              </select>
              <button type="submit" disabled={busy}>
                Add category
              </button>
            </form>
            <div className="cat-chips">
              {categories.length === 0 && <span className="muted">No categories yet.</span>}
              {categories.map((c) => (
                <span key={c.id} className="cat-chip">
                  {c.name}
                  {c.modifier ? ` (${c.modifier})` : ""}
                  <button onClick={() => handleDeleteCategory(c.id)} aria-label="delete category">
                    ×
                  </button>
                </span>
              ))}
            </div>
          </div>

          <div className="pool-board">
            <section className="pool-section" onDrop={onDropGeneric} onDragOver={allowDrop}>
              <div className="pool-section-head">
                <span className="pool-section-title">Generic pool</span>
                <span className="muted small">shared across all stages · drag a map into a category</span>
              </div>
              <form className="add-map" onSubmit={handleAddMap}>
                <input
                  value={mapId}
                  onChange={(e) => setMapId(e.target.value)}
                  placeholder="Beatmap id…"
                  inputMode="numeric"
                />
                <button type="submit" disabled={busy}>
                  Add map
                </button>
              </form>
              <div className="map-list">
                {generic.map((bm) => (
                  <MapCard
                    key={bm.beatmap_id}
                    bm={bm}
                    modifier={null}
                    drag={{ kind: "generic", beatmapId: bm.beatmap_id }}
                    onRemove={() => run(async () => {
                      await removeFromGenericPool(bm.beatmap_id);
                      await reload();
                    })}
                  />
                ))}
                {generic.length === 0 && <p className="muted small">Add maps by id — they'll be available in every stage.</p>}
              </div>
            </section>

            {categories.map((c) => {
              const items = entries.filter((en) => en.category_id === c.id);
              return (
                <section key={c.id} className="pool-section" onDrop={onDropCategory(c.id)} onDragOver={allowDrop}>
                  <div className="pool-section-head">
                    <span className="pool-section-title">{c.name}</span>
                    {c.modifier && <span className="mod-badge">{c.modifier}</span>}
                  </div>
                  <div className="map-list">
                    {items.map((en) => (
                      <MapCard
                        key={en.id}
                        bm={en}
                        modifier={c.modifier}
                        drag={{ kind: "entry", entryId: en.id }}
                        onRemove={() => run(async () => {
                          await deleteEntry(en.id);
                          await reload();
                        })}
                      />
                    ))}
                    {items.length === 0 && <p className="muted small">Drop maps here.</p>}
                  </div>
                </section>
              );
            })}
          </div>
        </>
      )}
    </div>
  );
}

function readDrag(e) {
  try {
    return JSON.parse(e.dataTransfer.getData("text/plain"));
  } catch {
    return null;
  }
}
