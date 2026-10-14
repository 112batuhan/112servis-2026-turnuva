import { useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import type { AdminUser, Role } from "../../api/types";
import { fetchUsers, updateUserRoles } from "../../api/users";
import { useSignedInUser } from "../../auth/AuthContext";
import { ROLE_LABELS, ROLES } from "../../auth/roles";
import { ErrorAlert } from "../../components/ErrorAlert";
import { formatDate, formatRank } from "../../lib/format";
import "./UsersPage.css";

export function UsersPage() {
  const me = useSignedInUser();
  const [users, setUsers] = useState<AdminUser[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savingId, setSavingId] = useState<string | null>(null);

  useEffect(() => {
    fetchUsers()
      .then(setUsers)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  /** Ticks or unticks one role; the rest stay as they are. */
  async function toggleRole(user: AdminUser, role: Role) {
    const roles = user.roles.includes(role)
      ? user.roles.filter((r) => r !== role)
      : ROLES.filter((r) => r === role || user.roles.includes(r));
    const previous = users;
    setError(null);
    setSavingId(user.id);
    setUsers((list) => list?.map((u) => (u.id === user.id ? { ...u, roles } : u)) ?? null);
    try {
      await updateUserRoles(user.id, roles);
    } catch (e) {
      setUsers(previous);
      setError(errorMessage(e));
    } finally {
      setSavingId(null);
    }
  }

  return (
    <section className="panel">
      <div className="page-header">
        <h1>Kayıtlı kullanıcılar</h1>
        {users && <span className="badge">{users.length}</span>}
      </div>

      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!users && !error && <p className="status">Yükleniyor…</p>}

      {users && (
        <div className="table-scroll">
          <table className="user-table">
            <thead>
              <tr>
                <th scope="col">
                  <span className="sr-only">Avatar</span>
                </th>
                <th scope="col">osu!</th>
                <th scope="col">Ülke</th>
                <th scope="col">Global rank</th>
                <th scope="col">Roller</th>
                <th scope="col">Discord</th>
                <th scope="col">Katılma tarihi</th>
              </tr>
            </thead>
            <tbody>
              {users.map((u) => (
                <tr key={u.id}>
                  <td>{u.avatar_url && <img className="row-avatar" src={u.avatar_url} alt="" loading="lazy" />}</td>
                  <td>
                    <a className="user-link" href={`https://osu.ppy.sh/users/${u.osu_id}`} target="_blank" rel="noopener noreferrer">
                      {u.username}
                    </a>{" "}
                    <span className="muted mono">#{u.osu_id}</span>
                  </td>
                  <td>{u.country_code ?? "—"}</td>
                  <td className="numeric">{formatRank(u.global_rank)}</td>
                  <td>
                    {/* Any set of roles; none means a regular player. A host can't drop their own Host. */}
                    <div className="role-toggles" role="group" aria-label={`${u.username} için roller`}>
                      {ROLES.map((r) => {
                        const on = u.roles.includes(r);
                        const ownHost = r === "host" && u.id === me.id;
                        return (
                          <button
                            key={r}
                            type="button"
                            className={`role-toggle role-${r}${on ? " is-on" : ""}`}
                            aria-pressed={on}
                            disabled={savingId === u.id || ownHost}
                            title={ownHost ? "Kendi host rolünü kaldıramazsın" : undefined}
                            onClick={() => toggleRole(u, r)}
                          >
                            {ROLE_LABELS[r]}
                          </button>
                        );
                      })}
                    </div>
                  </td>
                  <td>{u.discord_username ? `@${u.discord_username}` : <span className="muted">—</span>}</td>
                  <td>{formatDate(u.created_at)}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </section>
  );
}
