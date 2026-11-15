import { useEffect, useMemo, useRef, useState } from "react";
import { Link } from "react-router-dom";
import { loginWithOsu } from "../../api/auth";
import type {
  BracketMatch,
  CanPredict,
  MyPrediction,
  PickemStage,
  PickemStanding,
  PickemStatus,
} from "../../api/types";
import { useAuth } from "../../auth/AuthContext";
import { Countdown } from "../../components/Countdown";
import { useLinkedMatch } from "../../components/bracket/BracketViewTabs";
import { ErrorAlert } from "../../components/ErrorAlert";
import { StageTabs, type StageMark } from "../../components/StageTabs";
import { PickemPotentials } from "../../components/pickem/PickemPotentials";
import { PickemRow } from "../../components/pickem/PickemRow";
import { bestOfLabel } from "../../lib/bestOf";
import { compareByTime, incomingSources, isUndecided, potentialRows } from "../../lib/bracket";
import { canPredictMatch, defaultStage, isLocked, POINTS, STATUS_LABELS } from "../../lib/pickem";
import { PickemStandings } from "./PickemStandings";
import { usePickem } from "./usePickem";
import "./PickemPage.css";

type Tab = "ballot" | "standings";

/**
 * /pickem — predict every match of a stage before it starts. The stages run along the top, the
 * chosen one's matches are the ballot, and the standings sit beside it (a tab of their own on a
 * phone). Once a stage locks, each match shows how everyone predicted its winner.
 */
export function PickemPage() {
  const { user } = useAuth();
  const data = usePickem();
  const { overview, matches, teams, mine, splits } = data;
  const [picked, setPicked] = useState<string | null>(null);
  const [tab, setTab] = useState<Tab>("ballot");
  // Locked matches whose score distribution is open.
  const [expanded, setExpanded] = useState<Set<string>>(() => new Set());
  const toggle = (id: string) =>
    setExpanded((set) => {
      const next = new Set(set);
      if (!next.delete(id)) next.add(id);
      return next;
    });

  // The match page links here with ?match=: show its stage, open its distribution, bring it into view.
  const [linked, setLinked] = useState<string | null>(null);
  useLinkedMatch(setLinked);
  useEffect(() => {
    if (!linked || matches.length === 0) return;
    const m = matches.find((x) => x.id === linked);
    setLinked(null);
    if (!m?.stage) return;
    setPicked(m.stage.id);
    setTab("ballot");
    setExpanded((set) => new Set(set).add(m.id));
    requestAnimationFrame(() => {
      const row = document.getElementById(`pickem-${m.id}`);
      if (!row) return;
      row.scrollIntoView({
        block: "center",
        behavior: window.matchMedia("(prefers-reduced-motion: reduce)").matches ? "auto" : "smooth",
      });
      row.classList.add("is-linked");
    });
  }, [linked, matches]);

  const stages = useMemo(() => overview?.stages ?? [], [overview]);
  const stage = stages.find((s) => s.id === picked) ?? defaultStage(stages);
  const sources = useMemo(() => incomingSources(matches), [matches]);
  // The stage's matches in time order. A match still waiting on its week is predicted as each pairing
  // it may be, grouped under it and placed by its earliest pairing.
  const entries = useMemo(() => {
    if (!stage) return [];
    return matches
      .filter((m) => m.stage?.id === stage.id)
      .map((m) => {
        const potentials = isUndecided(m) ? potentialRows(m).sort(compareByTime) : [];
        return { match: m, potentials, lead: potentials[0] ?? m };
      })
      .sort((a, b) => compareByTime(a.lead, b.lead));
  }, [matches, stage]);
  // What can be predicted: each match, or each of its pairings.
  const rows = useMemo(
    () => entries.flatMap((e) => (e.potentials.length > 0 ? e.potentials : [e.match])),
    [entries],
  );
  const viewer = overview?.standings.find((s) => s.player.id === user?.id);
  const [railRef, railMore] = useMoreBelow(overview !== null && overview.stages.length > 0);
  const openStage = stages.find((s) => s.status === "open");

  if (!overview) {
    return (
      <div className="page pickem-page">
        <div className="page-header">
          <h1>Pick'em</h1>
        </div>
        <ErrorAlert message={data.error} onDismiss={data.clearError} />
        {!data.error && <p className="status">Yükleniyor…</p>}
      </div>
    );
  }

  return (
    <div className="page pickem-page" data-tab={tab}>
      <div className="page-header">
        <h1>Pick'em</h1>
        {overview.standings.length > 0 && (
          <span className="badge">{overview.standings.length} kişi</span>
        )}
        {openStage && openStage.id !== stage?.id && (
          <div className="page-header-aside">
            <button
              type="button"
              className="pickem-open-call"
              onClick={() => setPicked(openStage.id)}
            >
              <span className="pickem-open-call-name">{openStage.name}</span> tahmine açık
              {openStage.closes_at && (
                <Countdown
                  to={openStage.closes_at}
                  suffix="kaldı"
                  className="pickem-open-call-time"
                  onElapsed={data.reload}
                />
              )}
            </button>
          </div>
        )}
      </div>
      <ErrorAlert message={data.error} onDismiss={data.clearError} />

      {stages.length === 0 ? (
        <p className="status">Pick'em, bracket maçları stage'lere dağıtılınca açılır.</p>
      ) : (
        <>
          <StageTabs
            stages={stages}
            selectedId={stage?.id ?? null}
            onSelect={setPicked}
            mark={(s) => stageMark(s, viewer)}
            label="Pick'em stage'leri"
          />

          <div className="pickem-tabs" role="group" aria-label="Bölüm">
            <button
              type="button"
              className="pickem-tab"
              aria-pressed={tab === "ballot"}
              onClick={() => setTab("ballot")}
            >
              Tahminler
            </button>
            <button
              type="button"
              className="pickem-tab"
              aria-pressed={tab === "standings"}
              onClick={() => setTab("standings")}
            >
              Puan durumu
              {viewer && <span className="pickem-tab-rank num">{viewer.rank}.</span>}
            </button>
          </div>

          <div className="pickem-layout">
            {stage && (
              <section className="pickem-ballot" aria-labelledby="pickem-stage-title">
                <header className="pickem-stage-head">
                  <div className="pickem-stage-title-line">
                    <h2 id="pickem-stage-title">{stage.name}</h2>
                    {stage.best_of !== null && (
                      <span className="pickem-stage-bo">{bestOfLabel(stage.best_of)}</span>
                    )}
                    <span className={`pickem-stage-status is-${stage.status}`}>
                      {STATUS_LABELS[stage.status]}
                    </span>
                  </div>
                  <StageSummary stage={stage} viewer={viewer} rows={rows} mine={mine} />
                  <ul className="pickem-legend" aria-label="Puanlama">
                    <li>
                      <span className="num">{POINTS.exact}</span> kazanan ve skor
                    </li>
                    <li>
                      <span className="num">{POINTS.winner}</span> yalnız kazanan
                    </li>
                    <li>
                      <span className="num">{POINTS.score}</span> yalnız skor
                    </li>
                  </ul>
                </header>

                {stage.status === "open" && <AccountNotice canPredict={overview.can_predict} />}

                {rows.length === 0 ? (
                  <p className="status">Bu stage'de henüz maç yok.</p>
                ) : (
                  <ul className="pickem-list">
                    {entries.map(({ match, potentials }) => {
                      const row = (m: BracketMatch) => (
                        <PickemRow
                          key={m.id}
                          match={m}
                          team1={m.team1_id ? teams.get(m.team1_id) : undefined}
                          team2={m.team2_id ? teams.get(m.team2_id) : undefined}
                          sources={sources.get(m.id)}
                          stage={stage}
                          prediction={mine.get(m.id)}
                          split={splits.get(m.id)}
                          canPredict={overview.can_predict}
                          ownTeamId={data.bracket?.my_team_id}
                          onPredict={(teamId, loser) => data.predict(m, teamId, loser)}
                          onWithdraw={() => data.withdraw(m)}
                          expanded={expanded.has(m.id)}
                          onToggle={() => toggle(m.id)}
                        />
                      );
                      return potentials.length > 0 ? (
                        <PickemPotentials key={match.id} match={match}>
                          {potentials.map(row)}
                        </PickemPotentials>
                      ) : (
                        row(match)
                      );
                    })}
                  </ul>
                )}
              </section>
            )}

            <aside
              ref={railRef}
              className="pickem-rail"
              data-more={railMore || undefined}
              aria-label="Puan durumu"
            >
              <PickemStandings
                standings={overview.standings}
                stage={stage}
                viewerId={user?.id ?? null}
              />
            </aside>
          </div>
        </>
      )}
    </div>
  );
}

/** Whether a scrolling box has more below what it shows, so its bottom edge can fade out. */
function useMoreBelow(mounted: boolean) {
  const ref = useRef<HTMLElement>(null);
  const [more, setMore] = useState(false);
  useEffect(() => {
    const box = ref.current;
    if (!box) return;
    const update = () => setMore(box.scrollTop + box.clientHeight < box.scrollHeight - 1);
    update();
    box.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(box);
    for (const child of box.children) observer.observe(child);
    return () => {
      box.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, [mounted]);
  return [ref, more] as const;
}

/** Each window's rule on the track. */
const RULES: Record<PickemStatus, StageMark["rule"]> = {
  upcoming: undefined,
  open: "lit",
  locked: "set",
  finished: "done",
};

/**
 * A pick'em stage on the track: played out ruled in muted ink, locked in ink, the open one lit like
 * the crystal; under its name, the viewer's points once it's locked, else its state.
 */
function stageMark(s: PickemStage, viewer: PickemStanding | undefined): StageMark {
  const tally = viewer?.stages.find((t) => t.stage_id === s.id);
  const note = isLocked(s.status) && tally ? `${tally.points} puan` : STATUS_LABELS[s.status];
  return { rule: RULES[s.status], note };
}

/** The viewer's part in the stage: their points once it's locked, how far along they are while it's open. */
function StageSummary({
  stage,
  viewer,
  rows,
  mine,
}: {
  stage: PickemStage;
  viewer: PickemStanding | undefined;
  rows: BracketMatch[];
  mine: Map<string, MyPrediction>;
}) {
  if (stage.status === "open") {
    const open = rows.filter((m) => canPredictMatch(stage, m));
    if (open.length === 0) return null;
    const done = open.filter((m) => mine.has(m.id)).length;
    return (
      <p className={`pickem-stage-mine${done === open.length ? " is-complete" : ""}`}>
        <span className="pickem-stage-mine-figure">
          {done}/{open.length}
        </span>
        <span className="pickem-stage-mine-label">tahmin</span>
      </p>
    );
  }
  const tally = viewer?.stages.find((t) => t.stage_id === stage.id);
  if (!isLocked(stage.status) || !tally) return null;
  return (
    <p className="pickem-stage-mine">
      <span className="pickem-stage-mine-figure">{tally.points}</span>
      <span className="pickem-stage-mine-label">puan</span>
      <span className="pickem-stage-mine-detail">
        <span>{tally.exact} maçta skoru,</span>{" "}
        <span>{tally.winners} maçta kazananı tutturdun</span>
      </span>
    </p>
  );
}

/** What the viewer needs before they can predict. */
function AccountNotice({ canPredict }: { canPredict: CanPredict }) {
  if (canPredict === "yes") return null;
  if (canPredict === "banned") {
    return (
      <div className="pickem-notice">
        <p>Turnuvadan banlı olduğun için tahmin yapamazsın.</p>
      </div>
    );
  }
  return (
    <div className="pickem-notice">
      <p>
        {canPredict === "sign_in"
          ? "Tahmin yapmak için osu! hesabınla giriş yap."
          : "Tahmin yapmak için Discord hesabını bağlaman gerekiyor."}
      </p>
      {canPredict === "sign_in" ? (
        <button type="button" className="btn btn-primary btn-sm" onClick={loginWithOsu}>
          osu! ile giriş yap
        </button>
      ) : (
        <Link to="/discord" className="btn btn-discord btn-sm">
          Discord bağla
        </Link>
      )}
    </div>
  );
}
