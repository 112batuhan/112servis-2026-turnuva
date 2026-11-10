import { useState } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../../api/client";
import type { BracketMatch, CanPredict, MyPrediction, PickemSplit, PickemStage, Team } from "../../api/types";
import { fixturePath, isLive, matchCode, scoreHalves, sourceLabel, winningSlot, type Source } from "../../lib/bracket";
import { formatShortMatchTime } from "../../lib/format";
import { canPredictMatch, isMatchLocked, pointsLabel, predictionScore, predictionSlot } from "../../lib/pickem";
import { Icon } from "../Icon";
import { LiveMark } from "../LiveMark";
import { StatusChip } from "../StatusChip";
import { TeamBanner } from "../bracket/TeamBanner";
import { TeamPopover } from "../TeamPopover";
import { PickemClear } from "./PickemClear";
import { PickemSlider } from "./PickemSlider";
import { PickemScoreChart, PickemSplitBar } from "./PickemSplit";
import "./PickemRow.css";

interface Props {
  match: BracketMatch;
  team1: Team | undefined;
  team2: Team | undefined;
  /** What feeds each empty slot ("W3 kazananı"). */
  sources?: [Source | null, Source | null];
  stage: Pick<PickemStage, "status" | "best_of">;
  prediction: MyPrediction | undefined;
  split: PickemSplit | undefined;
  canPredict: CanPredict;
  ownTeamId?: string | null;
  onPredict: (teamId: string, loserScore: number) => Promise<void>;
  onWithdraw: () => Promise<void>;
  /** Whether the score distribution under a locked match's split is open. */
  expanded?: boolean;
  onToggle?: () => void;
}

/**
 * One match of a stage's pick'em as a line of the fixture sheet: the code and time, blue against
 * red with the match's score between them, and on the right what the viewer's prediction says.
 * While the stage is open the row is the ballot instead, as on the match page: the slider alone,
 * with the teams named at its ends (PickemSlider: pulled towards a side, the bigger that side's
 * win), so one gesture picks the winner and the score and the thumb is the only place the pick
 * shows. Once it locks, the teams and the score are back, the pick is said in words at the end,
 * and the split of everyone's winner picks sits under them, with the score distribution a click
 * away. A potential match is predicted like a match, but only counts if the match turns out to be
 * that pairing; it goes in a PickemPotentials under its match, and leads with "Olası" for the code.
 */
export function PickemRow({
  match,
  team1,
  team2,
  sources,
  stage,
  prediction,
  split,
  canPredict,
  ownTeamId,
  onPredict,
  onWithdraw,
  expanded = false,
  onToggle,
}: Props) {
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const open = canPredictMatch(stage, match);
  const locked = isMatchLocked(stage, match);
  const live = isLive(match);
  const potential = match.potential_of !== undefined;
  const slot = prediction ? predictionSlot(match, prediction) : null;
  const picked = prediction && slot ? predictionScore(slot, prediction) : null;
  const winner = winningSlot(match);

  async function run(key: string, action: () => Promise<void>) {
    setSaving(key);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(null);
    }
  }

  const side = (s: 1 | 2) => {
    const team = s === 1 ? team1 : team2;
    const source = sources?.[s - 1] ?? null;
    const classes = [
      "pickem-team",
      `is-slot-${s}`,
      winner !== null && winner !== s ? "is-loser" : "",
      team && team.id === ownTeamId ? "is-own" : "",
    ]
      .filter(Boolean)
      .join(" ");
    return (
      <span className={classes}>
        {team ? (
          // The banner on the inner side, by the score: after team 1's name, before team 2's. (The
          // trigger is one item, so the side's reversed row doesn't reach inside it.)
          <TeamPopover team={team}>
            {s === 2 && <TeamBanner team={team} />}
            <span className="pickem-team-name">{team.name}</span>
            {s === 1 && <TeamBanner team={team} />}
          </TeamPopover>
        ) : (
          <>
            <TeamBanner />
            <span className="pickem-team-name is-empty">{source ? sourceLabel(source) : "Belirlenmedi"}</span>
          </>
        )}
      </span>
    );
  };

  // The ballot's ends: each team's banner on its inner side, towards the other team, as in the
  // bracket list (team 1 name then banner, team 2 banner then name); the whole opens the team's card.
  const ballotName = (s: 1 | 2) => {
    const team = s === 1 ? team1 : team2;
    if (!team) return null;
    return (
      <TeamPopover team={team} className={team.id === ownTeamId ? "is-own" : undefined}>
        {s === 2 && team.banner_url && <TeamBanner team={team} />}
        <span className="pickem-slider-name">{team.name}</span>
        {s === 1 && team.banner_url && <TeamBanner team={team} />}
      </TeamPopover>
    );
  };

  // Between the teams only the match itself: its live or final score, else "vs".
  const halves = match.concluded_at || live ? scoreHalves(match) : null;
  const ahead = winner ?? (halves && halves[0] !== halves[1] ? (Number(halves[0]) > Number(halves[1]) ? 1 : 2) : null);
  const ballot = open && stage.best_of !== null && team1 !== undefined && team2 !== undefined;

  return (
    <li
      id={`pickem-${match.id}`}
      className={`pickem-row${ballot ? " is-ballot" : locked && split ? " has-extra" : ""}${live ? " is-live" : ""}${potential ? " is-potential" : ""}`}
    >
      <span className="pickem-row-when">
        {/* A potential match sits under its match's head (PickemPotentials), which carries the code. */}
        {potential ? (
          <StatusChip state="potential" title="Yalnız maç bu eşleşme olursa puan getirir">
            Olası
          </StatusChip>
        ) : (
          <Link to={fixturePath(match)} className={`pickem-row-code is-${match.side}`}>
            {matchCode(match)}
          </Link>
        )}
        {live ? (
          <LiveMark compact />
        ) : (
          <span className="pickem-row-time">{match.starts_at ? formatShortMatchTime(match.starts_at) : "Saat yok"}</span>
        )}
      </span>
      {!ballot && (
        <>
          {side(1)}
          <span
            className={`pickem-score${halves ? "" : " is-empty"}`}
            aria-label={halves ? (live ? "Canlı skor" : "Maç skoru") : undefined}
          >
            {halves ? (
              <>
                <span className={ahead === 1 ? "is-winner" : undefined}>{halves[0]}</span>
                <span className={ahead === 2 ? "is-winner" : undefined}>{halves[1]}</span>
              </>
            ) : (
              <span className="pickem-score-vs">vs</span>
            )}
          </span>
          {side(2)}
        </>
      )}
      <span className="pickem-row-result">
        <Result
          open={open}
          locked={locked}
          match={match}
          prediction={prediction}
          picked={picked}
          saving={saving}
          error={error}
          onWithdraw={() => run("withdraw", onWithdraw)}
        />
      </span>

      {ballot && stage.best_of !== null && team1 && team2 && (
        <PickemSlider
          bestOf={stage.best_of}
          teams={[team1, team2]}
          named
          renderName={ballotName}
          picked={slot && prediction ? { slot, loser: prediction.loser_score } : null}
          canPredict={canPredict}
          saving={saving !== null}
          label={`${matchCode(match)} skor tahmini`}
          onPick={(teamId, loser) => run("pick", () => onPredict(teamId, loser))}
          onClear={() => run("withdraw", onWithdraw)}
        />
      )}

      {locked && split && (
        <div className="pickem-row-split">
          <PickemSplitBar
            team1={split.team1}
            team2={split.team2}
            names={[team1?.name ?? "Takım 1", team2?.name ?? "Takım 2"]}
            mine={prediction?.void ? null : slot}
          />
          <div className="pickem-row-split-foot">
            <span className="pickem-row-total">
              <span className="num">{split.total}</span> tahmin
            </span>
            {split.total > 0 && stage.best_of !== null && onToggle && (
              <button
                type="button"
                className="pickem-row-toggle"
                aria-expanded={expanded}
                aria-controls={`pickem-chart-${match.id}`}
                onClick={onToggle}
              >
                Skor dağılımı
                <Icon name="chevronDown" size={14} />
              </button>
            )}
          </div>
          {expanded && stage.best_of !== null && (
            <div id={`pickem-chart-${match.id}`} className="pickem-row-chart">
              <PickemScoreChart
                bestOf={stage.best_of}
                scores={split.scores}
                names={[team1?.name ?? "Takım 1", team2?.name ?? "Takım 2"]}
                result={match.concluded_at && !match.forfeit_slot && match.team1_score !== null && match.team2_score !== null ? [match.team1_score, match.team2_score] : null}
                mine={picked && !prediction?.void ? picked : null}
              />
            </div>
          )}
        </div>
      )}
    </li>
  );
}

interface ResultProps {
  open: boolean;
  locked: boolean;
  match: BracketMatch;
  prediction: MyPrediction | undefined;
  picked: [number, number] | null;
  saving: string | null;
  error: string | null;
  onWithdraw: () => void;
}

/** The right end of the row: what the viewer's prediction is, or what it scored. */
function Result({ open, locked, match, prediction, picked, saving, error, onWithdraw }: ResultProps) {
  if (error) return <span className="pickem-result-error" role="alert">{error}</span>;
  if (open) {
    if (saving) return <span className="pickem-result-note">Kaydediliyor…</span>;
    if (!prediction) return <span className="pickem-result-note">Skoru seç</span>;
    return (
      <span className="pickem-result-saved">
        <span className="pickem-result-ok">
          <Icon name="check" size={14} />
          Kaydedildi
        </span>
        <PickemClear label={`${matchCode(match)} tahminini sil`} onClick={onWithdraw} />
      </span>
    );
  }
  if (!locked) {
    return <span className="pickem-result-note">{match.team1_id && match.team2_id ? "Açılınca" : "Takımlar belli değil"}</span>;
  }
  if (!prediction) return <span className="pickem-result-note">Tahmin yok</span>;
  if (prediction.void) return <span className="pickem-result-note">Tahminin geçersiz: takım değişti</span>;
  if (prediction.points !== null) {
    return (
      <span className={`pickem-points${prediction.points === 0 ? " is-zero" : ""}`}>
        <span className="pickem-points-figure">{prediction.points > 0 ? `+${prediction.points}` : "0"}</span>
        <span className="pickem-points-label">{pointsLabel(prediction.points)}</span>
        {picked && <Mine score={picked} />}
      </span>
    );
  }
  return (
    <span className="pickem-points is-pending">
      {picked ? <Mine score={picked} /> : <span className="pickem-points-label">Sonuç bekleniyor</span>}
    </span>
  );
}

/** "Senin tahminin 5–3", the score as the sides read it, at the end of a locked match's row. */
function Mine({ score }: { score: [number, number] }) {
  return (
    <span className="pickem-points-mine">
      Senin tahminin{" "}
      <span className="pickem-points-mine-score">
        {score[0]}–{score[1]}
      </span>
    </span>
  );
}
