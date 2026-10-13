import { useEffect, useState } from "react";
import { errorMessage } from "../../api/client";
import type { AdminUser, Role } from "../../api/types";
import { fetchUsers, updateUserRole } from "../../api/users";
import { ROLE_LABELS, ROLES } from "../../auth/roles";
import { ErrorAlert } from "../../components/ErrorAlert";
import { formatDate, formatRank } from "../../lib/format";
import "./UsersPage.css";

export function UsersPage() {
  const [users, setUsers] = useState<AdminUser[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [savingId, setSavingId] = useState<string | null>(null);

  useEffect(() => {
    fetchUsers()
      .then(setUsers)
      .catch((e) => setError(errorMessage(e)));
  }, []);

  async function changeRole(userId: string, role: Role) {
    const previous = users;
    setError(null);
    setSavingId(userId);
    setUsers((list) => list?.map((u) => (u.id === userId ? { ...u, role } : u)) ?? null);
    try {
      await updateUserRole(userId, role);
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
        <h1>Registered users</h1>
        {users && <span className="badge">{users.length}</span>}
      </div>

      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!users && !error && <p className="status">Loading…</p>}

      {users && (
        <div className="table-scroll">
          <table className="user-table">
            <thead>
              <tr>
                <th scope="col">
                  <span className="sr-only">Avatar</span>
                </th>
                <th scope="col">osu!</th>
                <th scope="col">Country</th>
                <th scope="col">Global rank</th>
                <th scope="col">Role</th>
                <th scope="col">Discord</th>
                <th scope="col">Joined</th>
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
                    <select
                      className={`select select-sm role-select role-${u.role}`}
                      value={u.role}
                      disabled={savingId === u.id}
                      aria-label={`Role for ${u.username}`}
                      onChange={(e) => changeRole(u.id, e.target.value as Role)}
                    >
                      {ROLES.map((r) => (
                        <option key={r} value={r}>
                          {ROLE_LABELS[r]}
                        </option>
                      ))}
                    </select>
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
