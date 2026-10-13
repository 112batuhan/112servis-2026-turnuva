import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Link } from "react-router-dom";
import { errorMessage } from "../api/client";
import { acceptInvite, deleteInvite, fetchTeamStatus, searchPlayers, sendInvite } from "../api/teams";
import type { Invite, PlayerResult, TeamStatus } from "../api/types";
import { useSignedInUser } from "../auth/AuthContext";
import { DiscordMark } from "../components/DiscordMark";
import { ErrorAlert } from "../components/ErrorAlert";
import { PlayerChip } from "../components/PlayerChip";
import { TeamCard } from "../components/TeamCard";
import { ELIGIBILITY_LABELS, eligibilityBadgeClass } from "../lib/eligibility";
import { formatRank, formatTeamValue } from "../lib/format";
import "./TeamPage.css";

const SEARCH_DELAY_MS = 300;

export function TeamPage() {
  const user = useSignedInUser();
  const isStaff = user.role !== "basic";
  const canForm = !isStaff && user.discord !== null;
  const [status, setStatus] = useState<TeamStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<PlayerResult[] | null>(null);

  const reload = useCallback(async () => {
    setStatus(await fetchTeamStatus());
  }, []);

  useEffect(() => {
    reload().catch((e) => setError(errorMessage(e)));
  }, [reload]);

  const runSearch = useCallback(async (q: string) => {
    setResults(q.trim() ? await searchPlayers(q) : null);
  }, []);

  useEffect(() => {
    const timer = setTimeout(() => runSearch(query).catch((e) => setError(errorMessage(e))), SEARCH_DELAY_MS);
    return () => clearTimeout(timer);
  }, [query, runSearch]);

  // Staff can't play, but someone promoted after forming a team still sees it.
  const staffOnly = isStaff && status !== null && !status.team;

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    }
    try {
      await Promise.all([reload(), runSearch(query)]);
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page page-narrow team-page">
      <div className="page-header">
        <h1>My team</h1>
      </div>
      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!status && !error && <p className="status">Loading…</p>}

      {staffOnly && (
        <section className="panel">
          <p className="status">Hosts and map poolers run the tournament, so they can't join a team.</p>
        </section>
      )}

      {status && !staffOnly && (
        <>
          <RankSummary status={status} />
          {!user.discord && !status.team && (
            <section className="panel team-discord-notice">
              <p>Link your Discord account to send and accept invites.</p>
              <Link to="/discord" className="btn btn-discord btn-sm">
                <DiscordMark />
                Link Discord
              </Link>
            </section>
          )}

          {status.team ? (
            <section className="panel">
              <h2>Your team</h2>
              <TeamCard team={status.team} />
            </section>
          ) : (
            <>
              <InviteList
                title="Invites for you"
                empty="Nobody has invited you yet."
                invites={status.received}
                actions={(invite) => (
                  <>
                    <button
                      type="button"
                      className="btn btn-primary btn-sm"
                      disabled={busy || !canForm || invite.eligibility !== "eligible"}
                      onClick={() => run(() => acceptInvite(invite.id))}
                    >
                      Accept
                    </button>
                    <button
                      type="button"
                      className="btn btn-secondary btn-sm"
                      disabled={busy}
                      onClick={() => run(() => deleteInvite(invite.id))}
                    >
                      Decline
                    </button>
                  </>
                )}
              />
              <InviteList
                title="Invites you sent"
                empty="You haven't invited anyone yet."
                invites={status.sent}
                actions={(invite) => (
                  <button
                    type="button"
                    className="btn btn-secondary btn-sm"
                    disabled={busy}
                    onClick={() => run(() => deleteInvite(invite.id))}
                  >
                    Cancel
                  </button>
                )}
              />

              <section className="panel">
                <h2>Find a teammate</h2>
                <input
                  className="input team-search"
                  type="search"
                  placeholder="Search by osu! username"
                  aria-label="Search players"
                  value={query}
                  onChange={(e) => setQuery(e.target.value)}
                />
                {results?.length === 0 && <p className="status">No players found.</p>}
                {results && results.length > 0 && (
                  <ul className="team-rows">
                    {results.map((p) => (
                      <li key={p.id} className="team-row">
                        <PlayerChip player={p} />
                        <RowMeta eligibility={p.eligibility} teamValue={p.team_value} />
                        <div className="team-row-actions">
                          {p.invite_received ? (
                            <span className="badge">Invited you</span>
                          ) : p.invite_sent ? (
                            <span className="badge">Invite sent</span>
                          ) : (
                            <button
                              type="button"
                              className="btn btn-primary btn-sm"
                              disabled={busy || !canForm || p.eligibility !== "eligible"}
                              onClick={() => run(() => sendInvite(p.id))}
                            >
                              Invite
                            </button>
                          )}
                        </div>
                      </li>
                    ))}
                  </ul>
                )}
              </section>
            </>
          )}
        </>
      )}
    </div>
  );
}

function RankSummary({ status }: { status: TeamStatus }) {
  return (
    <section className="panel team-summary">
      {status.global_rank && status.min_partner_rank ? (
        <p>
          Your rank is <strong>{formatRank(status.global_rank)}</strong>. Your teammate must be ranked{" "}
          <strong>{formatRank(status.min_partner_rank)}</strong> or lower so the team value stays within{" "}
          {formatTeamValue(status.limit)}.
        </p>
      ) : (
        <p>You don't have a global rank, so you can't form a team.</p>
      )}
    </section>
  );
}

function RowMeta({ eligibility, teamValue }: { eligibility: Invite["eligibility"]; teamValue: number | null }) {
  return (
    <div className="team-row-meta">
      <span className={eligibilityBadgeClass(eligibility)}>{ELIGIBILITY_LABELS[eligibility]}</span>
      <span className="muted" title="Team value together">
        {formatTeamValue(teamValue)}
      </span>
    </div>
  );
}

interface InviteListProps {
  title: string;
  empty: string;
  invites: Invite[];
  actions: (invite: Invite) => ReactNode;
}

function InviteList({ title, empty, invites, actions }: InviteListProps) {
  return (
    <section className="panel">
      <h2>
        {title} {invites.length > 0 && <span className="badge">{invites.length}</span>}
      </h2>
      {invites.length === 0 ? (
        <p className="status">{empty}</p>
      ) : (
        <ul className="team-rows">
          {invites.map((invite) => (
            <li key={invite.id} className="team-row">
              <PlayerChip player={invite.player} />
              <RowMeta eligibility={invite.eligibility} teamValue={invite.team_value} />
              <div className="team-row-actions">{actions(invite)}</div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
