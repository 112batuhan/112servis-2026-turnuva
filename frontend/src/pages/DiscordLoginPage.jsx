import { linkDiscord } from "../api.js";
import { useAuth } from "../AuthContext.jsx";
import DiscordMark from "../components/DiscordMark.jsx";

export default function DiscordLoginPage() {
  const { user } = useAuth();
  const discordLinked = Boolean(user.discord);

  return (
    <div className="content content-center">
      <div className="card">
        <h1>Log in with Discord</h1>
        {discordLinked ? (
          <p className="linked">Your Discord (@{user.discord.username}) is already linked.</p>
        ) : (
          <>
            <p className="subtitle">Link your Discord account to unlock Discord-gated features.</p>
            <button className="discord-btn" onClick={linkDiscord}>
              <DiscordMark />
              Log in with Discord
            </button>
          </>
        )}
      </div>
    </div>
  );
}
