import { useEffect, useState } from "react";
import { fetchPublicStages, fetchPublicStage } from "../api.js";
import PoolCategories from "../components/PoolCategories.jsx";

// Public, read-only map pool viewer. Shows only stages that have been published.
export default function PublicPoolPage() {
  const [stages, setStages] = useState([]);
  const [selectedId, setSelectedId] = useState(null);
  const [detail, setDetail] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    fetchPublicStages()
      .then((list) => {
        setStages(list);
        setSelectedId((cur) => cur ?? list[0]?.id ?? null);
      })
      .catch((e) => setError(e.message));
  }, []);

  useEffect(() => {
    if (!selectedId) {
      setDetail(null);
      return;
    }
    let active = true;
    fetchPublicStage(selectedId)
      .then((d) => active && setDetail(d))
      .catch((e) => active && setError(e.message));
    return () => {
      active = false;
    };
  }, [selectedId]);

  return (
    <div className="content mappool">
      <h1>Map pool</h1>
      {error && <p className="status status-error">{error}</p>}
      {!error && stages.length === 0 && <p className="status">No stages have been published yet.</p>}

      {stages.length > 0 && (
        <div className="stage-tabs">
          {stages.map((s) => (
            <button
              key={s.id}
              className={`stage-tab ${s.id === selectedId ? "stage-tab-active" : ""}`}
              onClick={() => setSelectedId(s.id)}
            >
              {s.name}
            </button>
          ))}
        </div>
      )}

      {detail && (
        <PoolCategories categories={detail.categories} slots={detail.slots} maps={detail.maps} />
      )}
    </div>
  );
}
