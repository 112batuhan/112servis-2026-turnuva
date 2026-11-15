import { useSearchParams } from "react-router-dom";
import { linkDiscord } from "../api/auth";
import { DISCORD_LINK_ERROR_TEXT, textFor } from "../api/errors";
import { useSignedInUser } from "../auth/AuthContext";
import { DiscordMark } from "../components/DiscordMark";

export function LinkDiscordPage() {
  const user = useSignedInUser();
  // Set by the backend when linking didn't go through.
  const [params] = useSearchParams();
  const error = params.get("error");

  return (
    <div className="card-page">
      <div className="card">
        <DiscordMark size={40} />
        <h1>Discord bağla</h1>
        {error && (
          <p className="alert alert-error" role="alert">
            {textFor(DISCORD_LINK_ERROR_TEXT, error, "failed")}
          </p>
        )}
        {user.discord?.in_guild === false ? (
          <>
            <p className="card-subtitle">
              <strong>@{user.discord.username}</strong> hesabın bağlı ama turnuvanın Discord
              sunucusunda değilsin. Maç kanalların orada; yeniden bağlarsan seni sunucuya geri
              ekleriz.
            </p>
            <button
              type="button"
              className="btn btn-discord btn-lg btn-block"
              onClick={linkDiscord}
            >
              <DiscordMark />
              Yeniden bağla ve sunucuya katıl
            </button>
          </>
        ) : user.discord ? (
          <p className="card-subtitle">
            <strong>@{user.discord.username}</strong> Discord hesabın zaten bağlı.
          </p>
        ) : (
          <>
            <p className="card-subtitle">
              Discord'a özel özellikleri açmak için Discord hesabını bağla.
            </p>
            <button
              type="button"
              className="btn btn-discord btn-lg btn-block"
              onClick={linkDiscord}
            >
              <DiscordMark />
              Discord ile devam et
            </button>
          </>
        )}
      </div>
    </div>
  );
}
