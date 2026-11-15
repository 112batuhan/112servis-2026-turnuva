import { useState, type FormEvent } from "react";
import { useSignedInUser } from "../../../auth/AuthContext";
import { Icon } from "../../../components/Icon";
import { useIrc } from "../../../irc/IrcContext";
import { ircName } from "../../../lib/bancho";
import { readString, removeString, writeString } from "../../../lib/storage";
import "./IrcLogin.css";

const USERNAME_KEY = "irc-username";
const PASSWORD_KEY = "irc-password";

/**
 * Logs into Bancho with the referee's own IRC credentials, or shows who's logged in. The room's bot
 * runs the room without it; this is for writing in the room as yourself (the bot makes you one of
 * its referees) and for private messages. A host writes in the room through its bot (`viaBot`, the
 * bot's name), so for them the form is folded away behind one line, for private messages only.
 */
export function IrcLogin({ viaBot = null }: { viaBot?: string | null }) {
  const user = useSignedInUser();
  const irc = useIrc();
  const [unfolded, setUnfolded] = useState(false);
  const [username, setUsername] = useState(
    () => readString(USERNAME_KEY) ?? ircName(user.username),
  );
  const [password, setPassword] = useState(() => readString(PASSWORD_KEY) ?? "");
  const [remember, setRemember] = useState(() => readString(PASSWORD_KEY) !== null);

  if (irc.status === "online") {
    return (
      <div className="irc-login is-online">
        <span className="irc-dot" aria-hidden="true" />
        <span className="irc-login-who">
          Bancho'ya <strong>{irc.nick}</strong> olarak bağlısın
        </span>
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          title="Bütün sekmelerde bağlantıyı keser"
          onClick={irc.disconnect}
        >
          Bağlantıyı kes
        </button>
      </div>
    );
  }

  if (viaBot !== null && !unfolded && irc.status !== "connecting") {
    return (
      <div className="irc-login is-online is-via-bot">
        <Icon name="bot" size={15} />
        <span className="irc-login-who">
          <strong>{viaBot || "Odanın botu"}</strong> üzerinden yazıyorsun
        </span>
        <button
          type="button"
          className="btn btn-secondary btn-sm"
          title="Kendi hesabınla, özel mesajlar için. Hesabın bir botsa bağlanamazsın."
          onClick={() => setUnfolded(true)}
        >
          Bancho'ya bağlan
        </button>
      </div>
    );
  }

  function submit(e: FormEvent) {
    e.preventDefault();
    const name = username.trim();
    if (!name || !password) return;
    writeString(USERNAME_KEY, name);
    if (remember) writeString(PASSWORD_KEY, password);
    else removeString(PASSWORD_KEY);
    irc.connect(name, password);
  }

  const connecting = irc.status === "connecting";
  return (
    <form className="irc-login" onSubmit={submit}>
      <h2 className="irc-login-title">Bancho'ya bağlan</h2>
      <p className="irc-login-why">
        Odayı bot yönetir. Odaya kendi hesabınla yazmak ve özel mesajlar için kendi IRC şifrenle
        bağlan; bot seni odanın hakemi yapar.
      </p>
      <input
        className="input input-sm"
        aria-label="osu! kullanıcı adı"
        placeholder="Kullanıcı adı"
        autoComplete="username"
        value={username}
        onChange={(e) => setUsername(e.target.value)}
      />
      <input
        className="input input-sm"
        type="password"
        aria-label="IRC şifresi"
        placeholder="IRC şifresi"
        autoComplete="current-password"
        value={password}
        onChange={(e) => setPassword(e.target.value)}
      />
      <label className="irc-login-remember">
        <input type="checkbox" checked={remember} onChange={(e) => setRemember(e.target.checked)} />
        Bu cihazda hatırla
      </label>
      <button
        type="submit"
        className="btn btn-primary btn-sm"
        disabled={connecting || !username.trim() || !password}
      >
        {connecting ? "Bağlanıyor…" : "Bağlan"}
      </button>
      <a
        className="irc-login-help"
        href="https://osu.ppy.sh/home/account/edit#legacy-api"
        target="_blank"
        rel="noopener noreferrer"
      >
        IRC şifreni buradan al
      </a>
      {irc.error && (
        <p className="irc-login-error" role="alert">
          {irc.error}
        </p>
      )}
    </form>
  );
}
