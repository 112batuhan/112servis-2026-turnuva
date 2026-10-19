import { useCallback, useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../../api/client";
import { fetchMatchPickem, predictMatch, withdrawPrediction } from "../../api/pickem";
import type { MatchPickem as MatchPickemData, PublicMatch } from "../../api/types";
import { Countdown } from "../../components/Countdown";
import { Icon } from "../../components/Icon";
import { PickemClear } from "../../components/pickem/PickemClear";
import { PickemSlider } from "../../components/pickem/PickemSlider";
import { PickemSplitBar } from "../../components/pickem/PickemSplit";
import { isLocked, pickemMatchPath, pointsLabel, STATUS_LABELS } from "../../lib/pickem";
import "./MatchPickem.css";

/**
 * The match page's pick'em panel, under the score sheet: while its stage is open, the ballot; once
 * it locks, the share that backed each team, with the viewer's own pick and what it scored. The
 * score distribution lives on the pick'em page, which the panel links to with this match open.
 * Nothing for a match outside every stage's pick'em.
 */
export function MatchPickem({ match }: { match: PublicMatch }) {
  const [data, setData] = useState<MatchPickemData | null>(null);
  const [saving, setSaving] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(() => fetchMatchPickem(match.id).then(setData, () => {}), [match.id]);
  // Read again whenever the match changes in a way that moves the pick'em.
  const key = [match.team1_id, match.team2_id, match.starts_at, match.rooms.length, match.concluded_at, match.forfeit_slot].join("|");
  useEffect(() => {
    load();
  }, [load, key]);

  if (match.kind !== "bracket" || !data) return null;
  const team1 = match.teams.find((t) => t.id === match.team1_id);
  const team2 = match.teams.find((t) => t.id === match.team2_id);
  const names: [string, string] = [team1?.name ?? "Takım 1", team2?.name ?? "Takım 2"];
  const bestOf = match.stage?.best_of ?? null;
  const mineSlot = data.mine && !data.mine.void ? (data.mine.team_id === match.team1_id ? 1 : 2) : null;

  async function pick(teamId: string, loser: number) {
    setSaving("pick");
    setError(null);
    try {
      await predictMatch(match.id, teamId, loser);
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(null);
    }
  }

  async function withdraw() {
    setSaving("withdraw");
    setError(null);
    try {
      await withdrawPrediction(match.id);
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setSaving(null);
    }
  }

  return (
    <section className="match-pickem" aria-labelledby="match-pickem-title">
      <header className="match-pickem-head">
        <div className="match-pickem-head-main">
          <h3 id="match-pickem-title" className="match-pickem-title">
            Pick'em
          </h3>
          <p className="match-pickem-status">
            <span>
              {data.stage_name} · <span className={`is-${data.status}`}>{STATUS_LABELS[data.status]}</span>
            </span>
            {data.status === "open" && data.closes_at && (
              <Countdown to={data.closes_at} suffix="kaldı" className="match-pickem-countdown" onElapsed={load} />
            )}
          </p>
        </div>
        <Link to={pickemMatchPath(match.id)} className="match-pickem-link">
          {isLocked(data.status) && data.split ? "Skor dağılımı" : "Pick'em sayfası"}
          <Icon name="arrowRight" size={14} />
        </Link>
      </header>

      {data.status === "upcoming" && <p className="match-pickem-note">Önceki stage bitince tahmine açılır.</p>}

      {data.status === "open" &&
        (data.predictable && team1 && team2 && bestOf !== null ? (
          <div className="match-pickem-ballot">
            <p className="match-pickem-ask">Kim, kaça kaç kazanır?</p>
            <PickemSlider
              bestOf={bestOf}
              teams={[team1, team2]}
              picked={mineSlot && data.mine ? { slot: mineSlot, loser: data.mine.loser_score } : null}
              canPredict={data.can_predict}
              saving={saving !== null}
              label="Skor tahmini"
              named
              onPick={pick}
              onClear={withdraw}
            />
            <div className="match-pickem-foot">
              {error ? (
                <span className="match-pickem-error" role="alert">
                  {error}
                </span>
              ) : data.can_predict === "sign_in" ? (
                <span>Tahmin için giriş yap.</span>
              ) : data.can_predict === "discord" ? (
                <span>
                  Tahmin için <Link to="/discord">Discord hesabını bağla</Link>.
                </span>
              ) : data.mine ? (
                <span className="match-pickem-saved">
                  <Icon name="check" size={14} />
                  Tahminin kaydedildi
                  <PickemClear label="Tahminini sil" disabled={saving !== null} onClick={withdraw} />
                </span>
              ) : (
                <span>Kazanacak takıma doğru çek: ne kadar uzağa, o kadar farklı. Maç başlayana kadar değiştirebilirsin.</span>
              )}
              <span className="match-pickem-count">
                <span className="num">{data.total}</span> kişi tahmin etti · dağılım ilk maçla açıklanır
              </span>
            </div>
          </div>
        ) : (
          <p className="match-pickem-note">Takımlar belli olunca tahmin edilebilir.</p>
        ))}

      {isLocked(data.status) && data.split && (
        <div className="match-pickem-reveal">
          <PickemSplitBar team1={data.split.team1} team2={data.split.team2} names={names} named mine={mineSlot} />
          <p className="match-pickem-mine">
            {!data.mine ? (
              <span className="match-pickem-note">Bu maç için tahmin yapmadın.</span>
            ) : data.mine.void ? (
              <span className="match-pickem-note">Tahmin ettiğin takım artık bu maçta değil.</span>
            ) : (
              <>
                <span>
                  Senin tahminin{" "}
                  <strong className="num">
                    {names[(mineSlot ?? 1) - 1]} {data.mine.winner_score}–{data.mine.loser_score}
                  </strong>
                </span>
                {data.mine.points !== null && (
                  <span className={`match-pickem-points${data.mine.points === 0 ? " is-zero" : ""}`}>
                    <span className="num">{data.mine.points > 0 ? `+${data.mine.points}` : "0"}</span>{" "}
                    {pointsLabel(data.mine.points)}
                  </span>
                )}
              </>
            )}
          </p>
        </div>
      )}

      {isLocked(data.status) && !data.split && (
        <p className="match-pickem-note">Stage kilitlendiğinde takımlar belli değildi; bu maç tahmin edilmedi.</p>
      )}
    </section>
  );
}
