import { useCallback, useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { fetchPublicBracket } from "../api/bracket";
import { errorMessage } from "../api/client";
import { fetchPublicStage, fetchPublicStages } from "../api/mappool";
import { fetchPublicQualifiers } from "../api/matches";
import { fetchStageActivity } from "../api/stats";
import type { BracketMatch, Category, Lobby, PublicStageDetail, Stage, StageActivity } from "../api/types";
import { ErrorAlert } from "../components/ErrorAlert";
import { Icon } from "../components/Icon";
import { MapCard } from "../components/MapCard";
import { PoolSection } from "../components/PoolSection";
import { EmptySlot, Slot } from "../components/Slot";
import { StageTabs } from "../components/StageTabs";
import { showsHrStats } from "../lib/bancho";
import { currentStageId } from "../lib/stageTrack";
import { useStoredSet } from "../lib/useExpandedSet";
import { useLiveBracket } from "../live/LiveContext";

/** Where the tournament is: its matches and lobbies (which stage it's in) and what's being played, by stage. */
interface Progress {
  matches: BracketMatch[];
  lobbies: Lobby[];
  activity: StageActivity[];
}

/** Each part degrades on its own: without the bracket there's no current stage, without the activity nothing is live. */
async function fetchProgress(): Promise<Progress> {
  const [bracket, qualifiers, activity] = await Promise.allSettled([
    fetchPublicBracket(),
    fetchPublicQualifiers(),
    fetchStageActivity(),
  ]);
  return {
    matches: bracket.status === "fulfilled" ? bracket.value.matches : [],
    lobbies: qualifiers.status === "fulfilled" ? qualifiers.value.lobbies : [],
    activity: activity.status === "fulfilled" ? activity.value : [],
  };
}

export function PublicPoolPage() {
  const [stages, setStages] = useState<Stage[] | null>(null);
  const [detail, setDetail] = useState<PublicStageDetail | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [fallback, setFallback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [searchParams, setSearchParams] = useSearchParams();
  // Every category starts open: the maps are what visitors come for. Only the ones they close are remembered.
  const { has: isCollapsed, toggle } = useStoredSet("public-pool-collapsed");

  // The open stage lives in the URL (?stage=…) so links to a stage can be shared.
  const requested = searchParams.get("stage");
  const selectedId = stages?.some((s) => s.id === requested) ? requested : fallback;
  const selected = stages?.find((s) => s.id === selectedId);

  // The stage the tournament is in (the qualifier's comes from the published stages), and the matches being played.
  const currentId = useMemo(
    () => (progress ? currentStageId(progress.matches, progress.lobbies, stages ?? []) : null),
    [progress, stages],
  );
  const live = useMemo(
    () => new Map(progress?.activity.filter((a) => a.live > 0).map((a) => [a.stage_id, a.live])),
    [progress],
  );

  const loadProgress = useCallback(() => {
    fetchProgress().then(setProgress);
  }, []);

  useEffect(() => {
    fetchPublicStages()
      .then(setStages)
      .catch((e) => setError(errorMessage(e)));
    loadProgress();
  }, [loadProgress]);

  // A room opening, a match ending or the bracket moving on can change which stage is lit.
  useLiveBracket(() => {}, loadProgress);

  // Without a stage in the URL, open the one the tournament is in, else one being played, else the
  // latest with scores. It's chosen once, so the page doesn't jump while it's being read.
  useEffect(() => {
    if (!stages || stages.length === 0 || !progress || fallback) return;
    const published = (id: string | null | undefined) => (id && stages.some((s) => s.id === id) ? id : null);
    const playing = progress.activity.find((a) => a.live > 0)?.stage_id;
    const played = [...progress.activity].reverse().find((a) => a.played > 0)?.stage_id;
    setFallback(published(currentId) ?? published(playing) ?? published(played) ?? stages[0].id);
  }, [stages, progress, currentId, fallback]);

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

      {stages === null && !error && <p className="status">Yükleniyor…</p>}
      {stages?.length === 0 && <p className="status">Henüz yayınlanmış bir stage yok.</p>}

      {stages && stages.length > 0 && (
        <StageTabs
          stages={stages}
          selectedId={selectedId}
          onSelect={(id) => setSearchParams({ stage: id })}
          currentId={currentId}
          live={live}
        >
          {selected?.pack_url && (
            <a
              className="btn btn-primary"
              href={selected.pack_url}
              target="_blank"
              rel="noopener noreferrer"
              title={`${selected.name} beatmap pack'ini indir`}
            >
              <Icon name="download" />
              Toplu indir
            </a>
          )}
        </StageTabs>
      )}

      {detail && detail.id === selectedId && (
        <div className="pool-board is-public">
          {detail.categories.length === 0 && <p className="status">Bu stage'de henüz kategori yok.</p>}
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
          <Slot key={slot.id} number={i + 1} label={`${category.name}${i + 1}`}>
            {map ? <MapCard map={map} showHr={showsHrStats(category.name)} /> : <EmptySlot>Daha sonra açıklanacak</EmptySlot>}
          </Slot>
        );
      })}
    </PoolSection>
  );
}
