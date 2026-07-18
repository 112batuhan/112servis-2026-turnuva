import { useEffect, useState } from "react";
import {
  fetchStages,
  createStage,
  deleteStage,
  fetchStage,
  setStagePublished,
  createCategory,
  deleteCategory,
  addSlot,
  updateSlotNotes,
  deleteSlot,
  addMap,
  moveMap,
  deleteMap,
  updateMapNotes,
} from "../api.js";
import MapCard from "../components/MapCard.jsx";
import EditableNote from "../components/EditableNote.jsx";
import { useCollapsed } from "../useCollapsed.js";

// Stable key for the generic pool section in the collapse set (categories use their id).
const GENERIC_POOL = "generic-pool";

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
  const [collapsed, toggleCollapsed] = useCollapsed("mappool-collapsed");

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

  const handleAddSlot = (categoryId) => run(async () => {
    await addSlot(categoryId);
    await reload();
  });

  const handleDeleteSlot = (slotId) => run(async () => {
    await deleteSlot(slotId);
    await reload();
  });

  // Save notes on blur; update local state so the field keeps its value without a reload.
  const handleUpdateSlotNotes = (slot, value) => {
    if (value === slot.editor_notes) return;
    setDetail((d) => ({
      ...d,
      slots: d.slots.map((s) => (s.id === slot.id ? { ...s, editor_notes: value } : s)),
    }));
    updateSlotNotes(slot.id, value).catch((e) => setError(e.message));
  };

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

  const handleMove = (mapPoolId, slotId) => run(async () => {
    await moveMap(mapPoolId, slotId);
    await reload();
  });

  // Save a map note on blur; update local state so the field keeps its value, no reload.
  const handleSaveMapNote = (mapPoolId, field, value) => {
    const map = maps.find((m) => m.id === mapPoolId);
    if (map && map[field] === value) return;
    setDetail((d) => ({
      ...d,
      maps: d.maps.map((m) => (m.id === mapPoolId ? { ...m, [field]: value } : m)),
    }));
    updateMapNotes(mapPoolId, { [field]: value }).catch((e) => setError(e.message));
  };

  // Drop onto a slot (slotId) assigns the map there; onto the generic pool (null) frees it.
  const onDrop = (slotId) => (e) => {
    e.preventDefault();
    const payload = readDrag(e);
    if (payload?.mapId) handleMove(payload.mapId, slotId);
  };

  const categories = detail?.categories ?? [];
  const slots = detail?.slots ?? [];
  const maps = detail?.maps ?? [];
  const generic = maps.filter((m) => !m.slot_id);

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
            {(() => {
              const isCollapsed = collapsed.has(GENERIC_POOL);
              return (
                <section
                  className={`pool-section ${isCollapsed ? "is-collapsed" : ""}`}
                  onDrop={onDrop(null)}
                  onDragOver={allowDrop}
                >
                  <div
                    className="pool-section-head pool-section-head-toggle"
                    onClick={() => toggleCollapsed(GENERIC_POOL)}
                  >
                    <span className="pool-section-title">Generic pool</span>
                    <span className="muted small">add maps by id + mods, then drag them into a category</span>
                    <span className="caret" aria-hidden="true">
                      ▾
                    </span>
                  </div>
                  <form className="add-map" onSubmit={handleAddMap} hidden={isCollapsed}>
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
                  <div className="map-list" hidden={isCollapsed}>
                    {generic.map((m) => (
                      <MapCard
                        key={m.id}
                        bm={m}
                        drag={{ mapId: m.id }}
                        onSaveNote={(field, value) => handleSaveMapNote(m.id, field, value)}
                        onRemove={() => run(async () => {
                          await deleteMap(m.id);
                          await reload();
                        })}
                      />
                    ))}
                    {generic.length === 0 && <p className="muted small">No maps in the generic pool.</p>}
                  </div>
                </section>
              );
            })()}

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
                    <span className="muted small">
                      {catSlots.length} slot{catSlots.length === 1 ? "" : "s"}
                    </span>
                    <button
                      className="slot-add"
                      onClick={(e) => {
                        e.stopPropagation();
                        handleAddSlot(c.id);
                      }}
                      disabled={busy}
                    >
                      + slot
                    </button>
                    <span className="caret" aria-hidden="true">
                      ▾
                    </span>
                  </div>
                  <div className="slot-grid" hidden={isCollapsed}>
                    {catSlots.length === 0 && <p className="muted small">Add slots to hold maps.</p>}
                    {catSlots.map((slot, i) => {
                      const slotMap = maps.find((m) => m.slot_id === slot.id);
                      return (
                        <div key={slot.id} className="slot" onDrop={onDrop(slot.id)} onDragOver={allowDrop}>
                          <span className="slot-label">#{i + 1}</span>
                          <div className="slot-body">
                            <div className="slot-head">
                              <EditableNote
                                value={slot.editor_notes}
                                placeholder="Editor notes…"
                                hidden
                                onSave={(v) => handleUpdateSlotNotes(slot, v)}
                              />
                              <button
                                className="slot-del"
                                onClick={() => handleDeleteSlot(slot.id)}
                                aria-label="remove slot"
                              >
                                ×
                              </button>
                            </div>
                            {slotMap ? (
                              <MapCard
                                bm={slotMap}
                                drag={{ mapId: slotMap.id }}
                                onSaveNote={(field, value) => handleSaveMapNote(slotMap.id, field, value)}
                                onRemove={() => handleMove(slotMap.id, null)}
                              />
                            ) : (
                              <div className="slot-empty muted small">Drop a map here</div>
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
        </>
      )}
    </div>
  );
}
