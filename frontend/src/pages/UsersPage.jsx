import { useEffect, useState } from "react";
import { fetchUsers, updateUserRole } from "../api.js";

const ROLES = ["host", "map_pooler", "basic"];
const ROLE_LABELS = { host: "Host", map_pooler: "Map Pooler", basic: "Basic" };

export default function UsersPage() {
  const [users, setUsers] = useState(null);
  const [error, setError] = useState(null);
  const [savingId, setSavingId] = useState(null);

  useEffect(() => {
    fetchUsers()
      .then(setUsers)
      .catch((e) => setError(e.message));
  }, []);

  async function handleRoleChange(userId, role) {
    setError(null);
    setSavingId(userId);
    const snapshot = users;
    // Optimistic: update the row now, revert if the request fails.
    setUsers((us) => us.map((u) => (u.id === userId ? { ...u, role } : u)));
    try {
      await updateUserRole(userId, role);
    } catch (e) {
      setUsers(snapshot);
      setError(e.message);
    } finally {
      setSavingId(null);
    }
  }

  return (
    <div className="panel">
      <div className="panel-head">
        <h1>Registered users</h1>
        {users && <span className="pill">{users.length}</span>}
      </div>

      {error && <p className="status status-error">{error}</p>}
      {!users && !error && <p className="status">Loading…</p>}

      {users && (
        <div className="table-scroll">
          <table className="user-table">
            <thead>
              <tr>
                <th></th>
                <th>osu!</th>
                <th>Country</th>
                <th>Global rank</th>
                <th>Role</th>
                <th>Discord</th>
                <th>Joined</th>
              </tr>
            </thead>
            <tbody>
              {users.map((u) => (
                <tr key={u.id}>
                  <td>{u.avatar_url && <img className="row-avatar" src={u.avatar_url} alt="" />}</td>
                  <td>
                    <a
                      className="user-link"
                      href={`https://osu.ppy.sh/users/${u.osu_id}`}
                      target="_blank"
                      rel="noreferrer"
                    >
                      {u.username}
                    </a>{" "}
                    <span className="muted">#{u.osu_id}</span>
                  </td>
                  <td>{u.country_code ?? "—"}</td>
                  <td>{u.global_rank ? `#${u.global_rank.toLocaleString()}` : "—"}</td>
                  <td>
                    <select
                      className={`role-select role-${u.role}`}
                      value={u.role}
                      disabled={savingId === u.id}
                      onChange={(e) => handleRoleChange(u.id, e.target.value)}
                    >
                      {ROLES.map((r) => (
                        <option key={r} value={r}>
                          {ROLE_LABELS[r]}
                        </option>
                      ))}
                    </select>
                  </td>
                  <td>
                    {u.discord_username ? (
                      <>
                        @{u.discord_username} <span className="muted">#{u.discord_id}</span>
                      </>
                    ) : (
                      "—"
                    )}
                  </td>
                  <td>{new Date(u.created_at).toLocaleDateString()}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
