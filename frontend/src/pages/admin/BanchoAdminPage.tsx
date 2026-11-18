import { useCallback, useEffect, useState, type FormEvent } from "react";
import { Link } from "react-router-dom";
import { createBot, deleteBot, fetchBots, setAutoRooms, updateBot } from "../../api/bancho";
import { errorMessage } from "../../api/client";
import type { BanchoBot, BotState, BotsView } from "../../api/types";
import { ErrorAlert } from "../../components/ErrorAlert";
import { Icon } from "../../components/Icon";
import { IconButton } from "../../components/IconButton";
import { Switch } from "../../components/Switch";
import { formatClock } from "../../lib/format";
import { useLive } from "../../live/LiveContext";
import "./BanchoAdminPage.css";

// While the page is visible it reads the bots this often, for their line counts; connections and
// rooms changing come over the live socket at once.
const POLL_MS = 5_000;

const STATE_LABELS: Record<BotState, string> = {
  online: "Bağlı",
  connecting: "Bağlanıyor…",
  off: "Kapalı",
  login_failed: "Giriş reddedildi",
  full: "Bancho dolu diyor",
};

// The connection's reasons (backend/src/bancho/bot.rs), as hosts read them.
const REASONS: Record<string, string> = {
  "could not reach Bancho": "Bancho'ya ulaşılamadı",
  "Bancho refused the login":
    "Bancho IRC şifresini kabul etmedi; şifreyi değiştirince yeniden dener",
  "Bancho closed the connection": "Bancho bağlantıyı kapattı",
  "Bancho didn't answer the login": "Bancho girişe cevap vermedi",
  "Bancho went quiet": "Bancho bir süredir sessizdi",
  "Bancho sent an invalid line": "Bancho geçersiz bir satır gönderdi",
};

/**
 * Host-only. The osu! accounts the site makes and runs match rooms with: each one's connection,
 * the rooms that count against its Bancho limit and how close it runs to its pace, with the way to
 * add one, change its IRC password, switch it off and on, or remove it.
 */
export function BanchoAdminPage() {
  const [view, setView] = useState<BotsView | null>(null);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      setView(await fetchBots());
    } catch (e) {
      setError(errorMessage(e));
    }
  }, []);

  useEffect(() => {
    load();
    const timer = window.setInterval(() => {
      if (document.visibilityState === "visible") load();
    }, POLL_MS);
    return () => window.clearInterval(timer);
  }, [load]);

  const { subscribe } = useLive();
  useEffect(
    () =>
      subscribe((event) => {
        if (event.type === "bancho") load();
      }),
    [subscribe, load],
  );

  const bots = view?.bots ?? [];
  const limit = view?.limits.rooms ?? 4;
  const active = bots.filter((b) => b.active);
  const online = active.filter((b) => b.status?.state === "online" || b.status?.state === "full");
  const used = online.reduce((n, b) => n + (b.status?.rooms.length ?? 0), 0);
  const capacity = online.length * limit;

  return (
    <section className="panel">
      <div className="page-header">
        <h1>Bancho botları</h1>
        {view && (
          <span
            className="badge"
            title={`${active.length} açık bottan ${online.length} tanesi bağlı`}
          >
            {online.length}/{active.length}
          </span>
        )}
        {view && online.length > 0 && (
          <div className="page-header-aside">
            <span className={`bancho-capacity${used >= capacity ? " is-full" : ""}`}>
              <span className="num">
                {used}/{capacity}
              </span>{" "}
              oda dolu
            </span>
          </div>
        )}
      </div>

      <ErrorAlert message={error} onDismiss={() => setError(null)} />
      {!view && !error && <p className="status">Yükleniyor…</p>}

      {view && (
        <>
          <p className="bancho-lead">
            Maç odalarını site bu osu! hesaplarıyla açar, yönetir ve kaydeder. Bancho bir hesabın
            aynı anda en fazla <span className="num">{limit}</span> odada olmasına izin verir
            (hakemi olduğu odalar dahil), bu yüzden botlar birbirinin odasına girmez; bir bot
            düşerse maç başka botun açacağı yeni odaya taşınır. Bütün botlar doluyken yeni oda
            açılmaz. Her bot <span className="num">{view.limits.window_secs}</span> saniyede en
            fazla <span className="num">{view.limits.lines}</span> satır gönderir.
          </p>

          <AutoRooms
            enabled={view.auto_rooms}
            onChange={(enabled) => {
              setView((v) => v && { ...v, auto_rooms: enabled });
              setAutoRooms(enabled).catch((e) => {
                setError(errorMessage(e));
                load();
              });
            }}
          />

          {bots.length === 0 ? (
            <p className="bancho-empty">
              Henüz bot yok. Hakemlerin oda açabilmesi için en az bir bot ekle: turnuvada oynamayan
              bir osu! hesabı ve onun IRC şifresi yeter.
            </p>
          ) : (
            <ul className="bancho-bots">
              {bots.map((bot) => (
                <BotRow key={bot.id} bot={bot} limit={limit} onChanged={load} onError={setError} />
              ))}
            </ul>
          )}

          <AddBot onAdded={load} onError={setError} />
        </>
      )}
    </section>
  );
}

function BotRow({
  bot,
  limit,
  onChanged,
  onError,
}: {
  bot: BanchoBot;
  limit: number;
  onChanged: () => void;
  onError: (message: string) => void;
}) {
  const [changing, setChanging] = useState(false);
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const status = bot.status;
  const state: BotState = status?.state ?? (bot.active ? "connecting" : "off");
  const rooms = status?.rooms ?? [];
  const reason = status?.error ? (REASONS[status.error] ?? status.error) : null;

  async function act(action: () => Promise<unknown>) {
    setBusy(true);
    try {
      await action();
      onChanged();
    } catch (e) {
      onError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  function toggle() {
    if (
      bot.active &&
      rooms.length > 0 &&
      !window.confirm(
        `${bot.username} kapatılsın mı? ${rooms.length} odası botsuz kalır: o maçlar başka bir botun açacağı yeni odayla sürer.`,
      )
    )
      return;
    act(() => updateBot(bot.id, { active: !bot.active }));
  }

  function remove() {
    if (
      !window.confirm(
        `${bot.username} silinsin mi? Odaları maçlarında kalır ama artık bir bot yönetmez.`,
      )
    )
      return;
    act(() => deleteBot(bot.id));
  }

  function savePassword(e: FormEvent) {
    e.preventDefault();
    if (!password) return;
    act(async () => {
      await updateBot(bot.id, { irc_password: password });
      setPassword("");
      setChanging(false);
    });
  }

  return (
    <li className={`bancho-bot is-${state}`}>
      <div className="bancho-bot-who">
        <span className="bancho-bot-dot" aria-hidden="true" />
        <span className="bancho-bot-text">
          <a
            className="bancho-bot-name"
            href={`https://osu.ppy.sh/users/${encodeURIComponent(bot.username)}`}
            target="_blank"
            rel="noopener noreferrer"
          >
            {bot.username}
          </a>
          <span className="bancho-bot-state">
            {STATE_LABELS[state]}
            {state === "online" &&
              status?.online_since &&
              ` · ${formatClock(status.online_since)}'den beri`}
          </span>
          {reason && <span className="bancho-bot-reason">{reason}</span>}
        </span>
      </div>

      <div className="bancho-bot-rooms">
        <span
          className="bancho-meter"
          role="img"
          aria-label={`${limit} odadan ${rooms.length} tanesi dolu`}
        >
          {Array.from({ length: limit }, (_, i) => (
            <span key={i} className={i < rooms.length ? "is-used" : undefined} />
          ))}
        </span>
        <span className="bancho-meter-figure num">
          {rooms.length}/{limit}
        </span>
        {rooms.length > 0 && (
          <ul className="bancho-room-list" aria-label="Odaları">
            {rooms.map((r) => (
              <li key={r.mp_id}>
                <Link
                  to={`/admin/referee/${r.match_id}`}
                  className={`bancho-room${r.access === "in" ? "" : " is-out"}`}
                  title={`#mp_${r.mp_id}${r.made ? "" : " · başka yerden açıldı, bot hakemi"}${r.access === "in" ? "" : " · bot odada değil"}`}
                >
                  <span className="num">{r.label ?? "Bitti"}</span>
                  {!r.made && <span className="bancho-room-tag">hakem</span>}
                </Link>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div
        className="bancho-bot-lines"
        title="Son 5 saniyede gönderilen ve sırada bekleyen satırlar"
      >
        {state === "online" || state === "full" ? (
          <>
            <span className="num">{status?.recent ?? 0}</span>
            <span className="bancho-bot-lines-label">satır</span>
            {(status?.queued ?? 0) > 0 && (
              <span className="bancho-bot-queue num">+{status?.queued} sırada</span>
            )}
          </>
        ) : (
          <span className="bancho-bot-lines-label">—</span>
        )}
      </div>

      <div className="bancho-bot-actions">
        <button
          type="button"
          className={`btn btn-sm ${bot.active ? "btn-secondary" : "btn-primary"}`}
          disabled={busy}
          onClick={toggle}
        >
          {bot.active ? "Kapat" : "Aç"}
        </button>
        <IconButton
          icon="pencil"
          label={`${bot.username}: IRC şifresini değiştir`}
          active={changing}
          aria-expanded={changing}
          onClick={() => setChanging((c) => !c)}
        />
        <IconButton
          icon="trash"
          label={`${bot.username} botunu sil`}
          disabled={busy}
          onClick={remove}
        />
      </div>

      {changing && (
        <form className="bancho-bot-password" onSubmit={savePassword}>
          <input
            className="input input-sm"
            type="password"
            autoComplete="new-password"
            aria-label={`${bot.username} için yeni IRC şifresi`}
            placeholder="Yeni IRC şifresi"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
          <button type="submit" className="btn btn-primary btn-sm" disabled={busy || !password}>
            Kaydet ve bağlan
          </button>
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            onClick={() => setChanging(false)}
          >
            Vazgeç
          </button>
        </form>
      )}
    </li>
  );
}

/** Adds an account: its osu! username and IRC password, connected at once unless asked not to. */
function AddBot({ onAdded, onError }: { onAdded: () => void; onError: (message: string) => void }) {
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [active, setActive] = useState(true);
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (!username.trim() || !password) return;
    setBusy(true);
    try {
      await createBot(username.trim(), password, active);
      setUsername("");
      setPassword("");
      onAdded();
    } catch (err) {
      onError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <form className="bancho-add" onSubmit={submit} aria-labelledby="bancho-add-title">
      <h2 id="bancho-add-title" className="bancho-add-title">
        Bot ekle
      </h2>
      <p className="bancho-add-hint">
        Hesap turnuvada oynamamalı. Hakemler odaya kendi hesaplarıyla bağlanır; bot onları odanın
        hakemi yapar.{" "}
        <a
          href="https://osu.ppy.sh/home/account/edit#legacy-api"
          target="_blank"
          rel="noopener noreferrer"
        >
          IRC şifresi hesabın ayarlarında
          <Icon name="arrowRight" size={12} />
        </a>
      </p>
      <div className="bancho-add-fields">
        <label className="bancho-add-field">
          <span>osu! kullanıcı adı</span>
          <input
            className="input"
            autoComplete="off"
            spellCheck={false}
            maxLength={15}
            value={username}
            onChange={(e) => setUsername(e.target.value)}
          />
        </label>
        <label className="bancho-add-field">
          <span>IRC şifresi</span>
          <input
            className="input"
            type="password"
            autoComplete="new-password"
            value={password}
            onChange={(e) => setPassword(e.target.value)}
          />
        </label>
        <label className="bancho-add-check">
          <input type="checkbox" checked={active} onChange={(e) => setActive(e.target.checked)} />
          Hemen bağlansın
        </label>
        <button
          type="submit"
          className="btn btn-primary"
          disabled={busy || !username.trim() || !password}
        >
          {busy ? "Ekleniyor…" : "Bot ekle"}
        </button>
      </div>
    </form>
  );
}

/**
 * The tournament-wide switch for rooms opening by themselves: on, the auto referee opens every
 * match's room 10 minutes before its time and Discord reminds the players 20 minutes before; off,
 * referees open the rooms, and the auto referee still runs the ones they open.
 */
function AutoRooms({
  enabled,
  onChange,
}: {
  enabled: boolean;
  onChange: (enabled: boolean) => void;
}) {
  return (
    <div className={`bancho-auto${enabled ? " is-on" : ""}`}>
      <Switch
        checked={enabled}
        onChange={(next) => {
          if (
            next &&
            !window.confirm(
              "Otomatik hakemdeki her maçın odası, maç saatinden 10 dakika önce botlarla açılacak ve oyunculara davet gidecek. Açılsın mı?",
            )
          )
            return;
          onChange(next);
        }}
      >
        Odaları otomatik aç
      </Switch>
      <p className="bancho-auto-text">
        {enabled
          ? "Açık: otomatik hakemdeki her maçın odası maç saatinden 10 dakika önce açılır, oyuncular davet edilir; 20 dakika önce Discord'dan hatırlatılır."
          : "Kapalı: odaları hakemler açar. Açılan odalarda otomatik hakem maçı yine yönetir."}
      </p>
    </div>
  );
}
