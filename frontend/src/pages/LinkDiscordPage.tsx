import { linkDiscord } from "../api/auth";
import { useSignedInUser } from "../auth/AuthContext";
import { DiscordMark } from "../components/DiscordMark";

export function LinkDiscordPage() {
  const user = useSignedInUser();

  return (
    <div className="card-page">
      <div className="card">
        <DiscordMark size={40} />
        <h1>Link Discord</h1>
        {user.discord ? (
          <p className="card-subtitle">
            Your Discord account <strong>@{user.discord.username}</strong> is already linked.
          </p>
        ) : (
          <>
            <p className="card-subtitle">Connect your Discord account to unlock Discord-only features.</p>
            <button type="button" className="btn btn-discord btn-lg btn-block" onClick={linkDiscord}>
              <DiscordMark />
              Continue with Discord
            </button>
          </>
        )}
      </div>
    </div>
  );
}
