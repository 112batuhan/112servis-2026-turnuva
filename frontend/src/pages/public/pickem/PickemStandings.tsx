import { useMemo, useState } from "react";
import type { PickemStage, PickemStanding, PickemTally, Player } from "../../api/types";
import { isLocked } from "../../lib/pickem";
import "../../components/bracket/BracketViewTabs.css";

const SHOWN = 20;

interface Row extends PickemTally {
  rank: number;
  player: Player;
}

/** Ranks rows by points, then exact predictions, then winners; level on points and exact share a rank. */
function rank(rows: Omit<Row, "rank">[]): Row[] {
  const sorted = [...rows].sort(
    (a, b) =>
      b.points - a.points ||
      b.exact - a.exact ||
      b.winners - a.winners ||
      a.player.username.localeCompare(b.player.username, "tr"),
  );
  const out: Row[] = [];
  sorted.forEach((r, i) => {
    const prev = out[i - 1];
    const same = prev && prev.points === r.points && prev.exact === r.exact;
    out.push({ ...r, rank: same ? prev.rank : i + 1 });
  });
  return out;
}

interface Props {
  standings: PickemStanding[];
  /** The stage the ballot shows, which the standings can narrow to. */
  stage: PickemStage | undefined;
  viewerId: string | null;
}

/**
 * The pick'em table: everyone who has predicted, by points, overall or in the stage the ballot
 * shows. The top of it, then the viewer's own row if they're further down.
 */
export function PickemStandings({ standings, stage, viewerId }: Props) {
  const [scope, setScope] = useState<"all" | "stage">("all");
  const [expanded, setExpanded] = useState(false);
  // A stage's table only means something once its predictions are in.
  const stageScoped = scope === "stage" && stage !== undefined && isLocked(stage.status);

  const rows = useMemo(() => {
    if (!stageScoped) return standings;
    return rank(
      standings.flatMap((s) => {
        const t = s.stages.find((x) => x.stage_id === stage.id);
        return t ? [{ ...t, player: s.player }] : [];
      }),
    );
  }, [standings, stageScoped, stage]);

  const viewerIndex = rows.findIndex((r) => r.player.id === viewerId);
  const viewer = viewerIndex >= 0 ? rows[viewerIndex] : null;
  const shown = expanded ? rows : rows.slice(0, SHOWN);
  const pinned = !expanded && viewerIndex >= SHOWN ? rows[viewerIndex] : null;

  return (
    <section className="pickem-standings" aria-labelledby="pickem-standings-title">
      <header className="pickem-standings-head">
        <h2 id="pickem-standings-title">Puan durumu</h2>
        {viewer && (
          <p className="pickem-standings-you">
            <span className="num">{viewer.rank}.</span> sıradasın ·{" "}
            <span className="num">{viewer.points}</span> puan
          </p>
        )}
        {stage && isLocked(stage.status) && (
          <div className="bracket-view-tabs" role="group" aria-label="Kapsam">
            <button
              type="button"
              className="bracket-view-tab"
              aria-pressed={!stageScoped}
              onClick={() => setScope("all")}
            >
              Genel
            </button>
            <button
              type="button"
              className="bracket-view-tab"
              aria-pressed={stageScoped}
              onClick={() => setScope("stage")}
            >
              {stage.name}
            </button>
          </div>
        )}
      </header>

      {rows.length === 0 ? (
        <p className="pickem-standings-empty">
          {stageScoped
            ? "Bu stage'de kimse tahmin yapmadı."
            : "Henüz kimse tahmin yapmadı. İlk tahmin senden olsun."}
        </p>
      ) : (
        <>
          <ol className="pickem-standings-list">
            {shown.map((r, i) => (
              <StandingRow
                key={r.player.id}
                row={r}
                own={r.player.id === viewerId}
                tied={i > 0 && shown[i - 1].rank === r.rank}
              />
            ))}
          </ol>
          {pinned && (
            <ol className="pickem-standings-list is-pinned" aria-label="Senin sıran">
              <StandingRow row={pinned} own tied={false} />
            </ol>
          )}
          {rows.length > SHOWN && (
            <button
              type="button"
              className="btn btn-secondary btn-sm pickem-standings-more"
              onClick={() => setExpanded((e) => !e)}
            >
              {expanded ? "Daha az göster" : `Tümünü göster (${rows.length})`}
            </button>
          )}
        </>
      )}
    </section>
  );
}

function StandingRow({ row, own, tied }: { row: Row; own: boolean; tied: boolean }) {
  return (
    <li className={own ? "pickem-standing is-own" : "pickem-standing"}>
      <span
        className={tied ? "pickem-standing-rank is-tied" : "pickem-standing-rank"}
        aria-label={`${row.rank}. sıra`}
      >
        {row.rank}
      </span>
      <img className="pickem-standing-avatar" src={row.player.avatar_url} alt="" loading="lazy" />
      <span className="pickem-standing-who">
        <a
          className="pickem-standing-name"
          href={`https://osu.ppy.sh/users/${row.player.osu_id}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          {row.player.username}
        </a>
        {own && <span className="pickem-standing-you">sen</span>}
        <span className="pickem-standing-meta">
          <span>{row.exact} maçta skoru,</span> <span>{row.winners} maçta kazananı tutturdu</span>
        </span>
      </span>
      <span className="pickem-standing-points" aria-label={`${row.points} puan`}>
        {row.points}
      </span>
    </li>
  );
}
