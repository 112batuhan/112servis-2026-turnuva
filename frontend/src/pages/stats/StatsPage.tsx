import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { errorMessage } from "../../api/client";
import { fetchPublicStages } from "../../api/mappool";
import type { Stage, StageActivity } from "../../api/types";
import { useAuth } from "../../auth/AuthContext";
import { ErrorAlert } from "../../components/ErrorAlert";
import { IconButton } from "../../components/IconButton";
import { bestOfLabel } from "../../lib/bestOf";
import { formatClock } from "../../lib/format";
import { MapBoard } from "./MapBoard";
import { PoolStats } from "./PoolStats";
import { Standings } from "./Standings";
import { buildIndex, isTab, ownTeamId, TABS, type StatsIndex, type StatsTab } from "./statsModel";
import { statsCache, useStageStats } from "./useStageStats";
import "./StatsPage.css";

/**
 * /stats — every published stage's statistics, counted from the scores the site saved as the
 * matches are played: the pool's picks, bans and best scores, each map's player and team
 * leaderboards, and the player and team standings, all measured in Z scores against the field
 * that played the same map. The qualifier's team standings are the bracket's seeding.
 */
export function StatsPage() {
  const { user } = useAuth();
  const [params, setParams] = useSearchParams();
  const [stages, setStages] = useState<Stage[] | null>(null);
  const [stagesError, setStagesError] = useState<string | null>(null);

  useEffect(() => {
    fetchPublicStages()
      .then(setStages)
      .catch((e) => setStagesError(errorMessage(e)));
  }, []);

  const requested = params.get("stage");
  const [fallback, setFallback] = useState<string | null>(null);
  const selectedId = stages?.some((s) => s.id === requested) ? requested : fallback;
  const stats = useStageStats(selectedId);

  // Without a stage in the URL, open the one being played, else the latest with scores.
  useEffect(() => {
    if (!stages || stages.length === 0 || fallback) return;
    if (!stats.activity) return;
    const live = stats.activity.find((a) => a.live > 0);
    const played = [...stats.activity].reverse().find((a) => a.played > 0);
    setFallback(live?.stage_id ?? played?.stage_id ?? stages[0].id);
  }, [stages, stats.activity, fallback]);

  // What's on screen is the stage whose statistics are in, which stays the last one until the
  // selected one's arrive: everything that describes them goes by it, only the track by the selection.
  const shown = stages?.find((s) => s.id === stats.shownId) ?? null;
  const index = useMemo(() => (stats.data ? buildIndex(stats.data) : null), [stats.data]);
  const requestedTab = params.get("tab");
  const tab: StatsTab = isTab(requestedTab) ? requestedTab : shown?.is_qualifier ? "teams" : "pool";

  const update = (changes: Record<string, string | null>) =>
    setParams(
      (prev) => {
        const next = new URLSearchParams(prev);
        for (const [k, v] of Object.entries(changes)) {
          if (v === null) next.delete(k);
          else next.set(k, v);
        }
        return next;
      },
      { replace: true },
    );

  const error = stagesError ?? stats.error;
  const ownTeam = index ? ownTeamId(index, user?.id) : null;

  return (
    <div className="page stats-page">
      <div className="page-header">
        <h1>İstatistik</h1>
        {stats.loadedAt && (
          <div className="page-header-aside stats-fresh">
            <span>
              Kaydedilen skorlardan, <span className="num">{formatClock(stats.loadedAt.toISOString())}</span> itibarıyla
            </span>
            <IconButton
              icon="refresh"
              label="Şimdi yenile"
              variant="outlined"
              disabled={stats.loading}
              onClick={() => void stats.reload()}
            />
          </div>
        )}
      </div>
      <ErrorAlert
        message={error}
        onDismiss={() => {
          setStagesError(null);
          stats.clearError();
        }}
      />

      {stages === null && !error && <p className="status">Yükleniyor…</p>}
      {stages?.length === 0 && <p className="status">Henüz yayınlanmış bir stage yok.</p>}

      {stages && stages.length > 0 && (
        <StageTrack
          stages={stages}
          activity={stats.activity}
          selectedId={selectedId}
          onSelect={(id) => update({ stage: id, map: null })}
          onPrefetch={statsCache.prefetch}
        />
      )}

      {shown && index && (
        <div className={stats.stale ? "stats-body is-stale" : "stats-body"} aria-busy={stats.stale}>
          <StageSummary stage={shown} index={index} />
          <nav className="stats-tabs" aria-label="İstatistik bölümleri">
            {TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                className="stats-tab"
                aria-pressed={t.id === tab}
                onClick={() => update({ tab: t.id })}
              >
                {t.label}
                <span className="stats-tab-count">{tabCount(t.id, index)}</span>
              </button>
            ))}
          </nav>
          {index.data.stats.plays.length === 0 && tab !== "teams" && tab !== "players" ? (
            <p className="stats-empty">
              {shown.is_qualifier
                ? "Henüz oynanmış bir lobi yok. Lobiler oynandıkça her map bittiğinde skorlar burada görünür."
                : "Bu stage'de henüz oynanmış bir map yok. Maçlar oynandıkça her map bittiğinde skorlar burada görünür."}
            </p>
          ) : (
            <TabPanel
              tab={tab}
              index={index}
              ownTeam={ownTeam}
              mapId={params.get("map")}
              onMap={(id) => update({ map: id })}
              onOpenMap={(id) => update({ tab: "map-players", map: id })}
            />
          )}
        </div>
      )}
      {selectedId && !index && !error && <p className="status">Yükleniyor…</p>}
    </div>
  );
}

function tabCount(tab: StatsTab, index: StatsIndex): number {
  switch (tab) {
    case "pool":
    case "map-players":
    case "map-teams":
      return index.entries.length;
    case "players":
      return index.data.stats.players.length;
    case "teams":
      return index.data.stats.teams.length;
  }
}

interface PanelProps {
  tab: StatsTab;
  index: StatsIndex;
  ownTeam: string | null;
  mapId: string | null;
  onMap: (id: string) => void;
  onOpenMap: (id: string) => void;
}

function TabPanel({ tab, index, ownTeam, mapId, onMap, onOpenMap }: PanelProps) {
  switch (tab) {
    case "pool":
      return <PoolStats index={index} ownTeam={ownTeam} onOpenMap={onOpenMap} />;
    case "map-players":
      return <MapBoard kind="players" index={index} ownTeam={ownTeam} mapId={mapId} onMap={onMap} />;
    case "map-teams":
      return <MapBoard kind="teams" index={index} ownTeam={ownTeam} mapId={mapId} onMap={onMap} />;
    case "players":
      return <Standings kind="players" index={index} ownTeam={ownTeam} />;
    case "teams":
      return <Standings kind="teams" index={index} ownTeam={ownTeam} />;
  }
}

interface TrackProps {
  stages: Stage[];
  activity: StageActivity[] | null;
  selectedId: string | null;
  onSelect: (id: string) => void;
  /** A stage about to be picked (hovered, focused): start reading it. */
  onPrefetch: (id: string) => void;
}

/**
 * The published stages as the track's rules: a stage with matches being played lit in frost,
 * one with scores in muted ink, one without any on a hairline. It wraps rather than scrolls.
 */
function StageTrack({ stages, activity, selectedId, onSelect, onPrefetch }: TrackProps) {
  return (
    <nav className="stats-track" aria-label="Stage'ler">
      <ol>
        {stages.map((s) => {
          const a = activity?.find((x) => x.stage_id === s.id);
          const state = a && a.live > 0 ? "is-live" : a && a.played > 0 ? "is-played" : "is-empty";
          const note =
            a && a.live > 0
              ? `${a.live} maç oynanıyor`
              : a && a.played > 0
                ? s.is_qualifier
                  ? `${a.played} lobi oynandı`
                  : `${a.played} maç oynandı`
                : "Henüz skor yok";
          return (
            <li key={s.id}>
              <button
                type="button"
                className={`stats-track-item ${state}`}
                aria-pressed={s.id === selectedId}
                onClick={() => onSelect(s.id)}
                onPointerEnter={() => onPrefetch(s.id)}
                onFocus={() => onPrefetch(s.id)}
              >
                <span className="stats-track-name">{s.name}</span>
                <span className="stats-track-note">{note}</span>
              </button>
            </li>
          );
        })}
      </ol>
    </nav>
  );
}

/** The stage's name and what's counted so far, with how the Z scores are worked out. */
function StageSummary({ stage, index }: { stage: Stage; index: StatsIndex }) {
  const { stats, matches } = index.data;
  const live = matches.filter((m) => m.live).length;
  return (
    <section className="stats-stage" aria-labelledby="stats-stage-title">
      <div className="stats-stage-line">
        <h2 id="stats-stage-title">{stage.name}</h2>
        {stage.best_of !== null && !stage.is_qualifier && (
          <span className="stats-stage-bo">{bestOfLabel(stage.best_of)}</span>
        )}
        <p className="stats-stage-facts">
          {stage.is_qualifier ? (
            <span>
              <span className="num">{stats.matches_played}</span>/<span className="num">{matches.length}</span> lobi
            </span>
          ) : (
            <span>
              <span className="num">{stats.matches_played}</span> maç
            </span>
          )}
          <span>
            <span className="num">{stats.teams.length}</span> takım
          </span>
          <span>
            <span className="num">{stats.plays.length}</span> takım skoru
          </span>
          {live > 0 && (
            <span className="stats-stage-live">
              <span className="num">{live}</span> maç oynanıyor
            </span>
          )}
        </p>
        <ZExplainer qualifier={stage.is_qualifier} />
      </div>
    </section>
  );
}

function ZExplainer({ qualifier }: { qualifier: boolean }) {
  return (
    <details className="stats-explain">
      <summary>Z skoru nasıl hesaplanıyor?</summary>
      <div className="stats-explain-body">
        <p>
          Her map'te, o map'i oynayanların skorlarının ortalaması ve standart sapması alınır. Bir skorun Z'si, o
          ortalamadan kaç standart sapma yukarıda (+) ya da aşağıda (−) olduğudur. Böylece zor ve kolay mapler aynı
          ağırlıkla sayılır: herkesin düşük aldığı bir mapte iyi oynamak, herkesin yüksek aldığı bir mapteki kadar
          değerlidir. Takımların Z'si iki oyuncunun toplam skorundan, oyuncularınki kendi skorlarından hesaplanır.
        </p>
        {qualifier ? (
          <>
            <p>
              <strong>Seeding:</strong> takımlar bütün maplerdeki Z'lerinin toplamına göre sıralanır; bracket'taki
              seed'leri bu sıradır. Bir map tekrar oynandıysa takımın son denemesi sayılır.
            </p>
            <p>
              Lobisi bitmiş bir takımın skoru olmayan map'i 0 puan sayılır: o map'te 0'ın alacağı Z verilir, ama 0
              map'in ortalamasına ve sapmasına katılmaz, böylece diğer takımların Z'si değişmez. Lobisi daha
              oynanmamış ya da sürmekte olan takımların eksik mapleri henüz sayılmaz.
            </p>
          </>
        ) : (
          <p>
            Sıralama, oynanan maplerdeki Z'lerin ortalamasıdır. Bir takım aynı map'i iki maçta oynadıysa ikisi de
            sayılır. Pick ve ban oranları, stage'de oynanmış maçlara göredir.
          </p>
        )}
      </div>
    </details>
  );
}
