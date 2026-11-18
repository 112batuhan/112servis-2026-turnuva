import { useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { errorMessage } from "../../api/client";
import { fetchPublicStage, fetchPublicStages } from "../../api/mappool";
import type { Category, PublicStage, PublicStageDetail } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { MapCard } from "../../components/MapCard";
import { PoolSection } from "../../components/PoolSection";
import { EmptySlot, Slot } from "../../components/Slot";
import { StageTabs } from "../../components/StageTabs";
import { showsHrStats, slotLabel } from "../../lib/category";
import { useStoredSet } from "../../lib/useExpandedSet";
import { createStageCache, useStageCache } from "../../lib/useStageCache";
import { useLiveBracket } from "../../live/LiveContext";

/** Every published pool as last read: a stage opened again shows at once while it's read again. */
const poolCache = createStageCache(fetchPublicStage);

export function PublicPoolPage() {
  // The published stages (only they have a pool to show), each with where the tournament is: its
  // state on the track and how many of its matches are played and being played.
  const [stages, setStages] = useState<PublicStage[] | null>(null);
  const [fallback, setFallback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  // Every category starts open: the maps are what visitors come for. Only the ones they close are remembered.
  const { has: isCollapsed, toggle } = useStoredSet("public-pool-collapsed");

  // The open stage lives in the URL (?stage=…) so links to a stage can be shared.
  const requested = searchParams.get("stage");
  const selectedId = stages?.some((s) => s.id === requested) ? requested : fallback;

  const currentId = stages?.find((s) => s.state === "current")?.id ?? null;
  const live = useMemo(
    () => new Map(stages?.filter((s) => s.live > 0).map((s) => [s.id, s.live])),
    [stages],
  );

  // A failed refresh keeps the stages shown; only the first reading reports.
  const loadStages = useCallback((initial: boolean) => {
    fetchPublicStages()
      .then((all) => setStages(all.filter((s) => s.published)))
      .catch((e) => initial && setError(errorMessage(e)));
  }, []);

  useEffect(() => loadStages(true), [loadStages]);

  // A room opening, a match ending or the bracket moving on can change which stage is lit.
  useLiveBracket(
    () => {},
    () => loadStages(false),
  );

  // Without a stage in the URL, open the one the tournament is in, else one being played, else the
  // latest with scores. It's chosen once, so the page doesn't jump while it's being read.
  useEffect(() => {
    if (!stages || stages.length === 0 || fallback) return;
    const playing = stages.find((s) => s.live > 0);
    const played = [...stages].reverse().find((s) => s.played > 0);
    setFallback((stages.find((s) => s.state === "current") ?? playing ?? played ?? stages[0]).id);
  }, [stages, fallback]);

  // The last pool shown stays until the selected one is in, dimmed if that takes a moment.
  const pool = useStageCache(poolCache, selectedId);
  const detail = pool.data;
  const shownError = error ?? pool.error;

  return (
    <div className="page">
      <div className="page-header">
        <h1>Mappool</h1>
      </div>
      <ErrorAlert
        message={shownError}
        onDismiss={() => {
          setError(null);
          pool.clearError();
        }}
      />

      {stages === null && !shownError && <p className="status">Yükleniyor…</p>}
      {stages?.length === 0 && <p className="status">Henüz yayınlanmış bir stage yok.</p>}

      {stages && stages.length > 0 && (
        <StageTabs
          stages={stages}
          selectedId={selectedId}
          onSelect={(id) => setSearchParams({ stage: id })}
          onPrefetch={poolCache.prefetch}
          currentId={currentId}
          live={live}
        >
          {/* The pack of the pool on screen: the stage list has no pack links, the pool does. */}
          {detail?.pack_url && (
            <a
              className="btn btn-primary"
              href={detail.pack_url}
              target="_blank"
              rel="noopener noreferrer"
              title={`${detail.name} beatmap pack'ini indir`}
            >
              <Icon name="download" />
              Toplu indir
            </a>
          )}
        </StageTabs>
      )}

      {detail && (
        <div
          className={pool.stale ? "pool-board is-public is-stale" : "pool-board is-public"}
          aria-busy={pool.stale}
        >
          {detail.categories.length === 0 && (
            <p className="status">Bu stage'de henüz kategori yok.</p>
          )}
          {detail.categories.map((category) => (
            <PublicCategory
              key={category.id}
              category={category}
              detail={detail}
              expanded={!isCollapsed(category.id)}
              onToggle={() => toggle(category.id)}
            />
          ))}
        </div>
      )}
      {selectedId && !detail && !shownError && <p className="status">Yükleniyor…</p>}
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
      meta={`${slots.length} map`}
      color={category.color}
      expanded={expanded}
      onToggle={onToggle}
    >
      {slots.length === 0 && <p className="pool-empty">Henüz map yok.</p>}
      {slots.map((slot, i) => {
        const map = detail.maps.find((m) => m.slot_id === slot.id);
        return (
          <Slot key={slot.id} number={i + 1} label={slotLabel(category, i, slots.length)}>
            {map ? (
              <MapCard map={map} showHr={showsHrStats(map, category.kind)} />
            ) : (
              <EmptySlot>Daha sonra açıklanacak</EmptySlot>
            )}
          </Slot>
        );
      })}
    </PoolSection>
  );
}
