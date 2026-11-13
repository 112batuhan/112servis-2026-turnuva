import { useCallback, useEffect, useState, type FormEvent, type ReactNode } from "react";
import { errorMessage } from "../api/client";
import { acceptReschedule, deleteReschedule, fetchMyReschedules, proposeReschedule } from "../api/teams";
import type { MyReschedules, RescheduleMatch, RescheduleProposal } from "../api/types";
import { matchCode } from "../lib/bracket";
import { formatMatchTime, fromDateTimeLocal, toDateTimeLocal } from "../lib/format";
import "./MatchReschedules.css";

interface Props {
  /** Called after a proposal is accepted, so whatever shows the match's time can reload. */
  onRescheduled: () => void;
  onError: (message: string) => void;
}

type Run = (action: () => Promise<void>, rescheduled?: boolean) => Promise<boolean>;

/**
 * The team's upcoming bracket matches with the times both teams have proposed for them. Either
 * player proposes; one player of the other team accepting a time moves the match there and
 * drops every other proposal for it. Potential matches are here too, so a team's second match of
 * the week can be set before they know who it's against.
 */
export function MatchReschedules({ onRescheduled, onError }: Props) {
  const [data, setData] = useState<MyReschedules | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(() => fetchMyReschedules().then(setData), []);

  useEffect(() => {
    load().catch((e) => onError(errorMessage(e)));
  }, [load, onError]);

  const run: Run = async (action, rescheduled = false) => {
    setBusy(true);
    let ok = true;
    try {
      await action();
      if (rescheduled) onRescheduled();
    } catch (e) {
      ok = false;
      onError(errorMessage(e));
    }
    try {
      await load();
    } catch (e) {
      onError(errorMessage(e));
    } finally {
      setBusy(false);
    }
    return ok;
  };

  const received = data?.matches.reduce((n, m) => n + (m.open ? m.received.length : 0), 0) ?? 0;

  return (
    <section className="panel">
      <h2>
        Maç saatini değiştir {received > 0 && <span className="badge badge-warning">{received}</span>}
      </h2>
      <p className="muted reschedule-hint">
        Rakibine istediğin kadar saat önerebilirsin. Önerilerden birini karşı takımdan bir oyuncunun kabul etmesi
        yeterli. Kabul ettiklerinde maç saati değişir.
      </p>
      {!data ? (
        <p className="status">Yükleniyor…</p>
      ) : data.matches.length === 0 ? (
        <p className="status">Rakibi belli olan, oynanmamış bir maçın yok.</p>
      ) : (
        <ul className="reschedule-list">
          {data.matches.map((m) => (
            <MatchCard key={m.id} match={m} limit={data.limit} busy={busy} run={run} />
          ))}
        </ul>
      )}
    </section>
  );
}

interface CardProps {
  match: RescheduleMatch;
  limit: number;
  busy: boolean;
  run: Run;
}

function MatchCard({ match, limit, busy, run }: CardProps) {
  const [value, setValue] = useState("");
  const full = match.sent.length >= limit;

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!value) return;
    if (await run(() => proposeReschedule(match.id, fromDateTimeLocal(value)))) setValue("");
  }

  function accept(p: RescheduleProposal) {
    const others = match.sent.length + match.received.length - 1;
    const message =
      `Maçı ${formatMatchTime(p.starts_at)} saatine almak istiyor musun?` +
      (others > 0 ? ` Diğer ${others} öneri iptal olacak.` : "");
    if (window.confirm(message)) run(() => acceptReschedule(p.id), true);
  }

  return (
    <li className={match.potential ? "reschedule-card is-potential" : "reschedule-card"}>
      <div className="reschedule-head">
        <span className={`reschedule-code is-${match.side}`}>{matchCode(match)}</span>
        {match.potential && <span className="badge badge-potential">Olası</span>}
        {match.stage_name && <span className="muted">{match.stage_name}</span>}
        <span className="reschedule-opponent">
          <span className="muted">vs</span> {match.opponent.name}
        </span>
      </div>
      {match.potential && (
        <p className="muted reschedule-note">
          Bu eşleşme bu haftaki maçların sonucuna bağlı. Saatini şimdiden ayarlayın: iki takım da {matchCode(match)}{" "}
          maçına gelirse maç burada anlaştığınız saatte oynanır.
        </p>
      )}
      <p className="reschedule-current">
        <span className="reschedule-label">Şu anki saat</span>
        {match.starts_at ? formatMatchTime(match.starts_at) : "Belirlenmedi"}
      </p>
      {!match.open && (
        <p className="muted reschedule-note">Maç başladı, saatini artık yalnızca yetkililer değiştirebilir.</p>
      )}

      <ProposalList
        title="Rakibin önerileri"
        empty="Rakibin henüz bir saat önermedi."
        proposals={match.received}
        actions={(p) => (
          <>
            <button
              type="button"
              className="btn btn-primary btn-sm"
              disabled={busy || !match.open}
              onClick={() => accept(p)}
            >
              Kabul et
            </button>
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy}
              onClick={() => run(() => deleteReschedule(p.id))}
            >
              Reddet
            </button>
          </>
        )}
      />
      <ProposalList
        title="Takımının önerileri"
        empty="Takımın henüz bir saat önermedi."
        proposals={match.sent}
        actions={(p) => (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={busy}
            onClick={() => run(() => deleteReschedule(p.id))}
          >
            İptal et
          </button>
        )}
      />

      {match.open && (
        <form className="reschedule-form" onSubmit={submit}>
          <input
            className="input"
            type="datetime-local"
            aria-label="Önerilecek saat"
            min={toDateTimeLocal(new Date().toISOString())}
            value={value}
            onChange={(e) => setValue(e.target.value)}
          />
          <button
            type="submit"
            className="btn btn-primary btn-sm"
            disabled={busy || !value || full}
            title={full ? `En fazla ${limit} öneri bekleyebilir, önce birini iptal et` : undefined}
          >
            Saat öner
          </button>
        </form>
      )}
    </li>
  );
}

interface ListProps {
  title: string;
  empty: string;
  proposals: RescheduleProposal[];
  actions: (p: RescheduleProposal) => ReactNode;
}

function ProposalList({ title, empty, proposals, actions }: ListProps) {
  return (
    <div className="reschedule-group">
      <h3 className="reschedule-label">
        {title} {proposals.length > 0 && <span className="num">{proposals.length}</span>}
      </h3>
      {proposals.length === 0 ? (
        <p className="reschedule-empty">{empty}</p>
      ) : (
        <ul className="reschedule-proposals">
          {proposals.map((p) => (
            <li key={p.id} className="reschedule-proposal">
              <span className="reschedule-time">{formatMatchTime(p.starts_at)}</span>
              <span className="muted reschedule-by">{p.proposed_by.username} önerdi</span>
              <span className="reschedule-actions">{actions(p)}</span>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}
