import { Link } from "react-router-dom";
import { useSignedInUser } from "../../auth/AuthContext";
import { canPlay, isStaff, PLAYER_LABEL, ROLE_LABELS } from "../../auth/roles";
import { DiscordMark } from "../../components/DiscordMark";
import { formatRank } from "../../lib/format";
import "./ProfilePage.css";

export function ProfilePage() {
  const user = useSignedInUser();

  return (
    <div className="page page-narrow profile-page">
      <div className="page-header profile-header">
        {user.avatar_url && <img className="profile-avatar" src={user.avatar_url} alt="" />}
        <h1>{user.username}</h1>
        <div className="profile-roles">
          {user.roles.length === 0 ? (
            <span className="badge">{PLAYER_LABEL}</span>
          ) : (
            user.roles.map((r) => (
              <span key={r} className="badge">
                {ROLE_LABELS[r]}
              </span>
            ))
          )}
        </div>
      </div>

      <dl className="profile-facts">
        <div>
          <dt>osu!</dt>
          <dd>
            <a
              href={`https://osu.ppy.sh/users/${user.osu_id}`}
              target="_blank"
              rel="noopener noreferrer"
            >
              Profilini aç
            </a>
          </dd>
        </div>
        <div>
          <dt>Global rank</dt>
          <dd className="num">{formatRank(user.global_rank)}</dd>
        </div>
        {user.country_rank && user.country_code && (
          <div>
            <dt>{user.country_code} rank</dt>
            <dd className="num">{formatRank(user.country_rank)}</dd>
          </div>
        )}
        <div>
          <dt>Discord</dt>
          <dd>
            {user.discord ? (
              <>@{user.discord.username}</>
            ) : (
              <span className="profile-discord">
                <span className="muted">Bağlı değil; takım kurmak için gerekli.</span>
                <Link className="btn btn-discord btn-sm" to="/discord">
                  <DiscordMark />
                  Discord bağla
                </Link>
              </span>
            )}
          </dd>
        </div>
      </dl>

      <div className="profile-links">
        {canPlay(user) && (
          <Link className="btn btn-secondary" to="/team">
            Takımım
          </Link>
        )}
        <Link className="btn btn-secondary" to="/bracket">
          Maçlar
        </Link>
        {isStaff(user) && (
          <Link className="btn btn-secondary" to="/admin">
            Yönetim paneli
          </Link>
        )}
      </div>
    </div>
  );
}
