import { useEffect, useState } from "react";
import { loginWithOsu, linkDiscord, fetchCurrentUser, logout } from "./api.js";

export default function App() {
  const [user, setUser] = useState(undefined); // undefined = loading, null = signed out
  const [error, setError] = useState(null);

  useEffect(() => {
    fetchCurrentUser()
      .then(setUser)
      .catch(() => setError("Couldn't reach the backend. Is it running on port 8080?"));
  }, []);

  async function handleLogout() {
    await logout();
    setUser(null);
  }

  return (
    <div className="page">
      <div className="card">
        {user === undefined && !error && <p className="status">Checking session…</p>}

        {error && <p className="status status-error">{error}</p>}

        {user === null && !error && (
          <>
            <h1>Sign in</h1>
            <p className="subtitle">Log in with your osu! account to continue.</p>
            <button className="osu-btn" onClick={loginWithOsu}>
              Log in with osu!
            </button>
          </>
        )}

        {user && (
          <>
            <img className="avatar" src={user.avatar_url} alt="" />
            <h1>Welcome, {user.username}</h1>
            <p className="subtitle">
              osu! id {user.osu_id}
              {user.global_rank && <> · #{user.global_rank.toLocaleString()}</>}
              {user.country_rank && user.country_code && (
                <> · {user.country_code} #{user.country_rank.toLocaleString()}</>
              )}
            </p>

            {user.discord ? (
              <p className="linked">Discord linked: @{user.discord.username}</p>
            ) : (
              <button className="discord-btn" onClick={linkDiscord}>
                <DiscordMark />
                Link Discord
              </button>
            )}

            <button className="logout-btn" onClick={handleLogout}>
              Log out
            </button>
          </>
        )}
      </div>
    </div>
  );
}

function DiscordMark() {
  return (
    <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
      <path d="M20.317 4.37a19.79 19.79 0 0 0-4.885-1.515.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0 12.64 12.64 0 0 0-.617-1.25.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057 19.9 19.9 0 0 0 5.993 3.03.078.078 0 0 0 .084-.028 14.09 14.09 0 0 0 1.226-1.994.076.076 0 0 0-.041-.106 13.107 13.107 0 0 1-1.872-.892.077.077 0 0 1-.008-.128 10.2 10.2 0 0 0 .372-.292.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127 12.3 12.3 0 0 1-1.873.892.076.076 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028 19.84 19.84 0 0 0 6.002-3.03.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.06.06 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.086-2.157-2.42 0-1.334.955-2.42 2.157-2.42 1.21 0 2.176 1.095 2.157 2.42 0 1.334-.955 2.42-2.157 2.42zm7.975 0c-1.183 0-2.157-1.086-2.157-2.42 0-1.334.955-2.42 2.157-2.42 1.21 0 2.176 1.095 2.157 2.42 0 1.334-.946 2.42-2.157 2.42z" />
    </svg>
  );
}
