import { useEffect, useState } from "react";
import { errorMessage } from "../api/client";
import { devClear, devLoginAs, devSeed, devSeedLive, devSeedQualifiers, devSeedTeams, fetchDevAccounts } from "../api/dev";
import type { DevAccount } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { ROLE_LABELS } from "../auth/roles";
import { formatRank } from "../lib/format";
import { ErrorAlert } from "./ErrorAlert";
import "./DevSwitcher.css";

// Floating account switcher for local testing. Only bundled when DEV_LOGIN=true,
// and still renders nothing if the backend doesn't expose /api/dev.
export function DevSwitcher() {
  const { user, signOut } = useAuth();
  const [accounts, setAccounts] = useState<DevAccount[] | null>(null);
  const [open, setOpen] = useState(false);
  const [filter, setFilter] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchDevAccounts()
      .then(setAccounts)
      .catch(() => setAccounts(null));
  }, []);

  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);

  if (!accounts) return null;

  // Every page keeps its own state, so a full reload is the simplest way to show the new account's view.
  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
      window.location.reload();
    } catch (e) {
      setError(errorMessage(e));
      setBusy(false);
    }
  }

  // The live test puts the signed-in account in a team, so it needs a real one.
  const real = accounts.some((a) => a.id === user?.id && !a.dummy);
  const needle = filter.trim().toLowerCase();
  const shown = needle ? accounts.filter((a) => a.username.toLowerCase().includes(needle)) : accounts;

  return (
    <div className="dev-switcher">
      {open && (
        <div className="dev-panel" role="dialog" aria-label="Switch test account">
          <div className="dev-panel-header">
            <strong>Switch account</strong>
            <span className="muted">{accounts.length} accounts</span>
          </div>
          <ErrorAlert message={error} onDismiss={() => setError(null)} />
          <input
            className="input dev-filter"
            type="search"
            placeholder="Filter by name"
            aria-label="Filter accounts"
            value={filter}
            autoFocus
            onChange={(e) => setFilter(e.target.value)}
          />
          <ul className="dev-accounts">
            {shown.map((a) => (
              <li key={a.id}>
                <button
                  type="button"
                  className={`dev-account ${a.id === user?.id ? "is-current" : ""}`}
                  disabled={busy || a.id === user?.id}
                  onClick={() => run(() => devLoginAs(a.id))}
                >
                  <img className="dev-account-avatar" src={a.avatar_url} alt="" loading="lazy" />
                  <span className="dev-account-name">{a.username}</span>
                  <span className="dev-account-rank">{formatRank(a.global_rank)}</span>
                  <span className="dev-account-tags">
                    {a.roles.map((r) => (
                      <span key={r} className="dev-tag dev-tag-role">
                        {ROLE_LABELS[r]}
                      </span>
                    ))}
                    {!a.dummy && <span className="dev-tag dev-tag-real">real</span>}
                    {a.in_team && <span className="dev-tag">team</span>}
                    {a.discord_linked && <span className="dev-tag">discord</span>}
                  </span>
                </button>
              </li>
            ))}
          </ul>
          <div className="dev-panel-actions">
            <button type="button" className="btn btn-secondary btn-sm" disabled={busy} onClick={() => run(devSeed)}>
              Reseed dummies
            </button>
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy}
              title="Reseed as the tournament just after its qualifier: 40 teams, every lobby played, no bracket yet (for /admin/seeding)"
              onClick={() => run(devSeedQualifiers)}
            >
              Reseed: qualifiers done
            </button>
            <button
              type="button"
              className="btn btn-secondary btn-sm"
              disabled={busy}
              title="Reseed with only seven teams and the dev staff: no lobby, score or bracket, so no qualifier places"
              onClick={() => run(devSeedTeams)}
            >
              Reseed: teams only
            </button>
            {real && (
              <button
                type="button"
                className="btn btn-secondary btn-sm"
                disabled={busy}
                title="Reseed, then put you in a team with a bracket match starting now, to play in a real osu! room"
                onClick={() => run(devSeedLive)}
              >
                Reseed + live match
              </button>
            )}
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={busy}
              onClick={() => window.confirm("Delete every dummy account, team and invite?") && run(devClear)}
            >
              Clear dummies
            </button>
            {user && (
              <button type="button" className="btn btn-secondary btn-sm" disabled={busy} onClick={() => run(signOut)}>
                Log out
              </button>
            )}
          </div>
        </div>
      )}
      <button
        type="button"
        className="dev-toggle"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        <span className="dev-toggle-label">DEV</span>
        {user ? user.username : "signed out"}
      </button>
    </div>
  );
}
