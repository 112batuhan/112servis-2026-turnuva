import { useEffect, useState } from "react";
import { fetchUsers } from "../api.js";

export default function AdminPage() {
  const [users, setUsers] = useState(null);
  const [error, setError] = useState(null);

  useEffect(() => {
    fetchUsers()
      .then(setUsers)
      .catch((e) => setError(e.message));
  }, []);

  return (
    <div className="content">
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
                      {u.username} <span className="muted">#{u.osu_id}</span>
                    </td>
                    <td>{u.country_code ?? "—"}</td>
                    <td>{u.global_rank ? `#${u.global_rank.toLocaleString()}` : "—"}</td>
                    <td>
                      <span className={`role role-${u.role}`}>{u.role}</span>
                    </td>
                    <td>{u.discord_username ? `@${u.discord_username}` : "—"}</td>
                    <td>{new Date(u.created_at).toLocaleDateString()}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
