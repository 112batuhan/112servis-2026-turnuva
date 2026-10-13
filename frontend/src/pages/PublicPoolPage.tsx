import { useEffect, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { errorMessage } from "../api/client";
import { fetchPublicStage, fetchPublicStages } from "../api/mappool";
import type { Category, PublicStageDetail, Stage } from "../api/types";
import { ErrorAlert } from "../components/ErrorAlert";
import { MapCard } from "../components/MapCard";
import { PoolSection } from "../components/PoolSection";
import { EmptySlot, Slot } from "../components/Slot";
import { StageTabs } from "../components/StageTabs";
import { plural } from "../lib/format";
import { useExpandedSet } from "../lib/useExpandedSet";

export function PublicPoolPage() {
  const [stages, setStages] = useState<Stage[] | null>(null);
  const [detail, setDetail] = useState<PublicStageDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  const { isExpanded, toggle } = useExpandedSet("public-pool-expanded");

  // The open stage lives in the URL (?stage=…) so links to a stage can be shared.
  const requested = searchParams.get("stage");
  const selectedId = stages?.some((s) => s.id === requested) ? requested : (stages?.[0]?.id ?? null);

  useEffect(() => {
    fetchPublicStages()
      .then(setStages)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  useEffect(() => {
    if (!selectedId) return;
    let active = true;
    fetchPublicStage(selectedId)
      .then((d) => active && setDetail(d))
      .catch((e) => active && setError(errorMessage(e)));
    return () => {
      active = false;
    };
  }, [selectedId]);

  return (
    <div className="page">
      <div className="page-header">
        <h1>Mappool</h1>
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />

      {stages === null && !error && <p className="status">Loading…</p>}
      {stages?.length === 0 && <p className="status">No stages have been published yet.</p>}

      {stages && stages.length > 0 && (
        <StageTabs stages={stages} selectedId={selectedId} onSelect={(id) => setSearchParams({ stage: id })} />
      )}

      {detail && detail.id === selectedId && (
        <div className="pool-board">
          {detail.categories.length === 0 && <p className="status">No categories in this stage yet.</p>}
          {detail.categories.map((category) => (
            <PublicCategory
              key={category.id}
              category={category}
              detail={detail}
              expanded={isExpanded(category.id)}
              onToggle={() => toggle(category.id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface PublicCategoryProps {
  category: Category;
  detail: PublicStageDetail;
  expanded: boolean;
  onToggle: () => void;
}

function PublicCategory({ category, detail, expanded, onToggle }: PublicCategoryProps) {
  const slots = detail.slots.filter((s) => s.category_id === category.id);

  return (
    <PoolSection
      title={category.name}
      meta={plural(slots.length, "map")}
      color={category.color}
      expanded={expanded}
      onToggle={onToggle}
    >
      {slots.length === 0 && <p className="pool-empty">No maps yet.</p>}
      {slots.map((slot, i) => {
        const map = detail.maps.find((m) => m.slot_id === slot.id);
        return (
          <Slot key={slot.id} number={i + 1}>
            {map ? <MapCard map={map} /> : <EmptySlot>To be announced</EmptySlot>}
          </Slot>
        );
      })}
    </PoolSection>
  );
}
