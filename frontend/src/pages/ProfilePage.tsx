import { Link } from "react-router-dom";
import { useSignedInUser } from "../auth/AuthContext";
import { ROLE_LABELS } from "../auth/roles";
import { DiscordMark } from "../components/DiscordMark";
import { formatRank } from "../lib/format";

export function ProfilePage() {
  const user = useSignedInUser();

  return (
    <div className="card-page">
      <div className="card">
        {user.avatar_url && <img className="card-avatar" src={user.avatar_url} alt="" />}
        <h1>{user.username}</h1>
        <span className="badge">{ROLE_LABELS[user.role]}</span>
        <p className="card-subtitle">
          osu! id {user.osu_id}
          {user.global_rank ? <> · {formatRank(user.global_rank)} global</> : null}
          {user.country_rank && user.country_code ? (
            <> · {user.country_code} {formatRank(user.country_rank)}</>
          ) : null}
        </p>

        <div className="card-section">
          {user.discord ? (
            <p>
              Discord hesabı bağlı: <strong>@{user.discord.username}</strong>
            </p>
          ) : (
            <>
              <p className="muted">Henüz Discord bağlamadın. Bazı özellikler için gerekli.</p>
              <Link className="btn btn-discord btn-block" to="/discord">
                <DiscordMark />
                Discord bağla
              </Link>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
