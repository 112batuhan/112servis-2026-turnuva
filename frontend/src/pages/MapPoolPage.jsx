import { useEffect, useState } from "react";
import {
  fetchStages,
  createStage,
  deleteStage,
  fetchStage,
  setStagePublished,
  createCategory,
  deleteCategory,
  addMap,
  moveMap,
  deleteMap,
} from "../api.js";
import MapCard from "../components/MapCard.jsx";

const MODIFIERS = ["", "HD", "HR", "DT", "HT", "EZ", "FL", "HDHR", "HDDT"];

const allowDrop = (e) => e.preventDefault();

function readDrag(e) {
  try {
    return JSON.parse(e.dataTransfer.getData("text/plain"));
  } catch {
    return null;
  }
}

export default function MapPoolPage() {
  const [stages, setStages] = useState([]);
  const [selectedId, setSelectedId] = useState(null);
  const [detail, setDetail] = useState(null);
  const [error, setError] = useState(null);
  const [busy, setBusy] = useState(false);

  const [newStage, setNewStage] = useState("");
  const [mapId, setMapId] = useState("");
  const [mapMods, setMapMods] = useState("");
  const [catName, setCatName] = useState("");

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

  const handleTogglePublish = () => run(async () => {
    await setStagePublished(selectedId, !detail.published);
    await reload();
    await loadStages();
  });

  const handleAddCategory = (e) => {
    e.preventDefault();
    const name = catName.trim();
    if (!name || !selectedId) return;
    run(async () => {
      await createCategory(selectedId, name);
      setCatName("");
      await reload();
    });
  };

  const handleDeleteCategory = (id) => run(async () => {
    await deleteCategory(id);
    await reload();
  });

  // Add a map by id + mods. It lands in the generic pool with its stats locked in.
  const handleAddMap = (e) => {
    e.preventDefault();
    const id = parseInt(mapId, 10);
    if (!id) return;
    run(async () => {
      await addMap(id, mapMods);
      setMapId("");
      await reload();
    });
  };

  const handleMove = (mapPoolId, categoryId) => run(async () => {
    await moveMap(mapPoolId, categoryId);
    await reload();
  });

  const onDrop = (categoryId) => (e) => {
    e.preventDefault();
    const payload = readDrag(e);
    if (payload?.mapId) handleMove(payload.mapId, categoryId);
  };

  const categories = detail?.categories ?? [];
  const maps = detail?.maps ?? [];
  const generic = maps.filter((m) => !m.category_id);

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
              <button type="submit" disabled={busy}>
                Add category
              </button>
            </form>
            <div className="cat-chips">
              {categories.length === 0 && <span className="muted">No categories yet.</span>}
              {categories.map((c) => (
                <span key={c.id} className="cat-chip">
                  {c.name}
                  <button onClick={() => handleDeleteCategory(c.id)} aria-label="delete category">
                    ×
                  </button>
                </span>
              ))}
            </div>
          </div>

          <div className="pool-board">
            <section className="pool-section" onDrop={onDrop(null)} onDragOver={allowDrop}>
              <div className="pool-section-head">
                <span className="pool-section-title">Generic pool</span>
                <span className="muted small">add maps by id + mods, then drag them into a category</span>
              </div>
              <form className="add-map" onSubmit={handleAddMap}>
                <input
                  value={mapId}
                  onChange={(e) => setMapId(e.target.value)}
                  placeholder="Beatmap id…"
                  inputMode="numeric"
                />
                <select value={mapMods} onChange={(e) => setMapMods(e.target.value)}>
                  {MODIFIERS.map((m) => (
                    <option key={m} value={m}>
                      {m === "" ? "NoMod" : m}
                    </option>
                  ))}
                </select>
                <button type="submit" disabled={busy}>
                  Add map
                </button>
              </form>
              <div className="map-list">
                {generic.map((m) => (
                  <MapCard
                    key={m.id}
                    bm={m}
                    drag={{ mapId: m.id }}
                    onRemove={() => run(async () => {
                      await deleteMap(m.id);
                      await reload();
                    })}
                  />
                ))}
                {generic.length === 0 && <p className="muted small">No maps in the generic pool.</p>}
              </div>
            </section>

            {categories.map((c) => {
              const items = maps.filter((m) => m.category_id === c.id);
              return (
                <section key={c.id} className="pool-section" onDrop={onDrop(c.id)} onDragOver={allowDrop}>
                  <div className="pool-section-head">
                    <span className="pool-section-title">{c.name}</span>
                  </div>
                  <div className="map-list">
                    {items.map((m) => (
                      <MapCard
                        key={m.id}
                        bm={m}
                        drag={{ mapId: m.id }}
                        onRemove={() => handleMove(m.id, null)}
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
