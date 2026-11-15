import { type FormEvent, useState } from "react";
import { guessArg } from "../api/arg";
import { linkDiscord, loginWithOsu } from "../api/auth";
import { errorMessage } from "../api/client";
import { useAuth } from "../auth/AuthContext";
import { DiscordMark } from "../components/DiscordMark";
import "./ArgPage.css";

/** The hidden page behind the navbar's trophy. The server checks every answer and only
 *  sends a message's reply in answer to the guess that entered it, so nothing here gives
 *  the answers away, and a reply is gone once the page is left or the next guess is sent. */
export function ArgPage() {
  const { user } = useAuth();
  const ready = !!user?.discord;

  return (
    <div className="card-page">
      <div className="card kupa-card">
        <span className="brand-mark kupa-mark" aria-hidden="true" />
        {user === undefined ? null : ready ? (
          <Riddle />
        ) : (
          // The input is shown locked so it's clear what signing in is for. Discord can
          // only be linked once signed in with osu!, so only the next step's button shows.
          <>
            <div className="kupa-form">
              <input className="input kupa-input" disabled aria-label="Cevap" />
              <button type="button" className="btn btn-primary btn-block" disabled>
                Gönder
              </button>
            </div>
            <p className="kupa-locked">
              {user
                ? "Mesaj yollayabilmek için Discord hesabını bağla."
                : "Mesaj yollayabilmek için osu! ile giriş yap ve Discord hesabını bağla."}
            </p>
            {user ? (
              <button
                type="button"
                className="btn btn-discord btn-lg btn-block"
                onClick={linkDiscord}
              >
                <DiscordMark />
                Discord hesabını bağla
              </button>
            ) : (
              <button
                type="button"
                className="btn btn-primary btn-lg btn-block"
                onClick={loginWithOsu}
              >
                osu! ile giriş yap
              </button>
            )}
          </>
        )}
      </div>
    </div>
  );
}

function Riddle() {
  const [reply, setReply] = useState<string | null>(null);
  const [guess, setGuess] = useState("");
  const [busy, setBusy] = useState(false);
  const [wrong, setWrong] = useState(0);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!guess.trim() || busy) return;
    setBusy(true);
    setError(null);
    try {
      const result = await guessArg(guess);
      setReply(result.reply);
      if (result.correct) {
        setGuess("");
        setWrong(0);
      } else {
        // Counting rather than flagging restarts the shake on every miss.
        setWrong((n) => n + 1);
      }
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <>
      {reply && <p className="kupa-reply">{reply}</p>}
      <form className="kupa-form" onSubmit={submit}>
        <input
          key={wrong}
          className={wrong > 0 ? "input kupa-input is-wrong" : "input kupa-input"}
          value={guess}
          onChange={(e) => setGuess(e.target.value)}
          maxLength={500}
          autoComplete="off"
          spellCheck={false}
          aria-label="Cevap"
          autoFocus
        />
        <button
          type="submit"
          className="btn btn-primary btn-block"
          disabled={busy || !guess.trim()}
        >
          Gönder
        </button>
      </form>
      {error && <p className="kupa-error">{error}</p>}
    </>
  );
}
