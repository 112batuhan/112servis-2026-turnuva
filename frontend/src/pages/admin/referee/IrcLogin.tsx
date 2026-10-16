import { useState, type FormEvent } from "react";
import { useSignedInUser } from "../../../auth/AuthContext";
import { useIrc } from "../../../irc/IrcContext";
import { ircName } from "../../../lib/bancho";
import { readString, removeString, writeString } from "../../../lib/storage";
import "./IrcLogin.css";

const USERNAME_KEY = "irc-username";
const PASSWORD_KEY = "irc-password";

/** Logs into Bancho with the referee's own IRC credentials, or shows who's logged in. */
export function IrcLogin() {
  const user = useSignedInUser();
  const irc = useIrc();
  const [username, setUsername] = useState(() => readString(USERNAME_KEY) ?? ircName(user.username));
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
      <p className="irc-login-why">Odayı yönetmek ve sohbete yazmak için kendi osu! IRC hesabınla.</p>
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
      <button type="submit" className="btn btn-primary btn-sm" disabled={connecting || !username.trim() || !password}>
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
