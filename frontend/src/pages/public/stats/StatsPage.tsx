import { useEffect, useMemo, useState } from "react";
import { useSearchParams } from "react-router-dom";
import { errorMessage } from "../../../api/client";
import { fetchPublicStages } from "../../../api/mappool";
import type { PublicStage, StageActivity } from "../../../api/types";
import { useAuth } from "../../../auth/AuthContext";
import { ErrorAlert } from "../../../components/ErrorAlert";
import { IconButton } from "../../../components/IconButton";
import { StageTabs, type StageMark } from "../../../components/StageTabs";
import { bestOfLabel } from "../../../lib/bestOf";
import { formatClock } from "../../../lib/format";
import { MapBoard } from "./MapBoard";
import { PoolStats } from "./PoolStats";
import { Standings } from "./Standings";
import {
  buildIndex,
  isTab,
  ownTeamId,
  TABS,
  ToolsSlot,
  type StatsIndex,
  type StatsTab,
} from "./statsModel";
import { statsCache, useStageStats } from "./useStageStats";
import "./StatsPage.css";

/**
 * /stats — every published stage's statistics, counted from the scores the site saved as the
 * matches are played: the pool's picks, bans and best scores, each map's player and team
 * leaderboards, and the player and team standings, all measured in points (Z-percentiles) against the field
 * that played the same map. The qualifier's team standings are the bracket's seeding.
 */
export function StatsPage() {
  const { user } = useAuth();
  const [params, setParams] = useSearchParams();
  const [stages, setStages] = useState<PublicStage[] | null>(null);
  const [stagesError, setStagesError] = useState<string | null>(null);
  const [toolsSlot, setToolsSlot] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    // Only a published stage has statistics to show; the rest are on the landing page's track.
    fetchPublicStages()
      .then((all) => setStages(all.filter((s) => s.published)))
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
              Kaydedilen skorlardan,{" "}
              <span className="num">{formatClock(stats.loadedAt.toISOString())}</span> itibarıyla
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
        <StageTabs
          stages={stages}
          mark={(s) => stageMark(s, stats.activity)}
          selectedId={selectedId}
          onSelect={(id) => update({ stage: id, map: null })}
          onPrefetch={statsCache.prefetch}
        />
      )}

      {shown && index && (
        <div className={stats.stale ? "stats-body is-stale" : "stats-body"} aria-busy={stats.stale}>
          <StageSummary stage={shown} index={index} />
          <div className="stats-bar">
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
            <div className="stats-tools" ref={setToolsSlot} />
          </div>
          {index.data.stats.plays.length === 0 && tab !== "teams" && tab !== "players" ? (
            <p className="stats-empty">
              {shown.is_qualifier
                ? "Henüz oynanmış bir lobi yok. Lobiler oynandıkça her map bittiğinde skorlar burada görünür."
                : "Bu stage'de henüz oynanmış bir map yok. Maçlar oynandıkça her map bittiğinde skorlar burada görünür."}
            </p>
          ) : (
            <ToolsSlot.Provider value={toolsSlot}>
              <TabPanel
                tab={tab}
                index={index}
                ownTeam={ownTeam}
                mapId={params.get("map")}
                onMap={(id) => update({ map: id })}
                onOpenMap={(id) => update({ tab: "map-players", map: id })}
              />
            </ToolsSlot.Provider>
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
      return (
        <MapBoard kind="players" index={index} ownTeam={ownTeam} mapId={mapId} onMap={onMap} />
      );
    case "map-teams":
      return <MapBoard kind="teams" index={index} ownTeam={ownTeam} mapId={mapId} onMap={onMap} />;
    case "players":
      return <Standings kind="players" index={index} ownTeam={ownTeam} />;
    case "teams":
      return <Standings kind="teams" index={index} ownTeam={ownTeam} />;
  }
}

/**
 * A published stage on the track: lit in frost while its matches are being played, ruled in muted
 * ink once it has scores, on a hairline before.
 */
function stageMark(s: PublicStage, activity: StageActivity[] | null): StageMark {
  const a = activity?.find((x) => x.stage_id === s.id);
  if (a && a.live > 0) return { rule: "lit", note: `${a.live} maç oynanıyor` };
  if (a && a.played > 0)
    return { rule: "done", note: `${a.played} ${s.is_qualifier ? "lobi" : "maç"} oynandı` };
  return { note: "Henüz skor yok" };
}

/** The stage's name and what's counted so far, with how the points are worked out. */
function StageSummary({ stage, index }: { stage: PublicStage; index: StatsIndex }) {
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
              <span className="num">{stats.matches_played}</span>/
              <span className="num">{matches.length}</span> lobi
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
      <summary>
        {qualifier ? "Puan ve seed nasıl hesaplanıyor?" : "Puan nasıl hesaplanıyor?"}
      </summary>
      <div className="stats-explain-body">
        {qualifier && (
          <section className="stats-explain-part" aria-labelledby="stats-explain-seed">
            <h3 id="stats-explain-seed">Sıralama puanı</h3>
            <p className="stats-explain-lead">
              Seed'i belirleyen puandır: bir takımın her map'te aldığı puanların (Z-yüzdelik)
              toplamı.
            </p>
            <p className="stats-formula">sıralama puanı = Σ map puanı − eksik map sayısı</p>
            <dl className="stats-explain-scale">
              <div>
                <dt>0–1</dt>
                <dd>
                  Skoru olan her map'in puanı. Geçilemeyen (fail) skorlar da olduğu gibi sayılır.
                </dd>
              </div>
              <div>
                <dt>−1</dt>
                <dd>
                  Lobisi bitmiş bir takımın hiç skoru olmayan map'i: toplamdan 1 puan düşülür. Eksik
                  map o map'in ortalamasına ve sapmasına katılmaz, böylece kimsenin puanını
                  değiştirmez.
                </dd>
              </div>
            </dl>
            <p>
              Tablodaki Puan bu toplamdır; bracket'taki seed'ler bu sıradır. Bir map en çok 1 puan
              verdiği için tek bir map'teki çok yüksek skor sıralamayı tek başına belirlemez.
            </p>
            <p>
              Bir map tekrar oynandıysa takımın son denemesi sayılır. Lobisi daha oynanmamış ya da
              sürmekte olan takımların eksik mapleri henüz sayılmaz.
            </p>
          </section>
        )}
        <section className="stats-explain-part" aria-labelledby="stats-explain-points">
          <h3 id="stats-explain-points">Puan (Z-yüzdelik)</h3>
          <p className="stats-explain-lead">
            Bir skorun, o map'teki skorların yaklaşık ne kadarını geride bıraktığıdır: 0 ile 1
            arasında bir sayı.
            {!qualifier && " Sıralama, oynanan maplerdeki puanların ortalamasına göredir."}
          </p>
          <p>
            Önce skorun map ortalamasından kaç standart sapma yukarıda ya da aşağıda olduğuna (Z)
            bakılır:
          </p>
          <p className="stats-formula">Z = (skor − ortalama) / standart sapma</p>
          <p className="stats-formula">σ = √( Σ(skor − ortalama)² / (skor sayısı − 1) )</p>
          <p>
            Sonra Z, standart normal dağılımda o noktanın altında kalan paya çevrilir (Φ, hesap
            tablolarındaki NORMSDIST):
          </p>
          <p className="stats-formula">puan = Φ(Z)</p>
          <dl className="stats-explain-scale">
            <div>
              <dt>0,84</dt>
              <dd>Ortalamanın bir standart sapma üstünde bir skor.</dd>
            </div>
            <div>
              <dt>0,50</dt>
              <dd>Tam map'in ortalaması kadar bir skor.</dd>
            </div>
            <div>
              <dt>0,16</dt>
              <dd>Ortalamanın bir standart sapma altında bir skor.</dd>
            </div>
          </dl>
          <p>
            Ortalamaya yakın skorlardaki farklar puanı en çok değiştirir; çok yüksek ya da çok düşük
            skorlarda puan 1'e ya da 0'a yaklaşır ve daha fazla değişmez.
          </p>
        </section>
      </div>
    </details>
  );
}
