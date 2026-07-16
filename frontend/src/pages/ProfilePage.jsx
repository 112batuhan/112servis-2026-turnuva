import { Link } from "react-router-dom";
import { useAuth } from "../AuthContext.jsx";

export default function ProfilePage() {
  const { user } = useAuth();
  const discordLinked = Boolean(user.discord);

  return (
    <div className="content content-center">
      <div className="card">
        {user.avatar_url && <img className="avatar" src={user.avatar_url} alt="" />}
        <h1>Welcome, {user.username}</h1>
        <p className="subtitle">
          osu! id {user.osu_id}
          {user.global_rank ? <> · #{user.global_rank.toLocaleString()}</> : null}
          {user.country_rank && user.country_code ? (
            <>
              {" "}
              · {user.country_code} #{user.country_rank.toLocaleString()}
            </>
          ) : null}
        </p>

        {discordLinked ? (
          <p className="linked">Discord linked: @{user.discord.username}</p>
        ) : (
          <div className="notice">
            <p className="notice-text">
              You haven't linked your Discord account yet. Some features will require it.
            </p>
            <Link className="discord-btn" to="/discord">
              Log in with Discord
            </Link>
          </div>
        )}
      </div>
    </div>
  );
}
