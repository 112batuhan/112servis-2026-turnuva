import { useCallback, useEffect, useRef, useState } from "react";
import { Link, useLocation } from "react-router-dom";
import { errorMessage } from "../api/client";
import {
  confirmNotification,
  declineNotification,
  fetchNotificationCount,
  fetchNotifications,
  markNotificationsRead,
} from "../api/notifications";
import type { AppNotification, MatchStaffRole } from "../api/types";
import { useAuth } from "../auth/AuthContext";
import { hasRole } from "../auth/roles";
import { RELOAD_DELAY_MS, useLive } from "../live/LiveContext";
import { bracketMatchPath, matchCode } from "../lib/bracket";
import { formatDateTime, formatRank, formatShortMatchTime, formatTeamValue } from "../lib/format";
import { IconButton } from "./IconButton";
import "./NotificationBell.css";

/** Only while the live socket is down; otherwise it says when the count changes. */
const POLL_MS = 60_000;

const ROLE_AS: Record<MatchStaffRole, string> = { referee: "hakem", streamer: "yayıncı" };

/**
 * Every signed-in user's notifications: a bell with the number that need attention (unread, or
 * waiting for an answer) and a panel that lists them. Opening the panel marks them read; a
 * referee or streamer asked to confirm a match's new time answers right there, and a player's
 * notices link to their team page.
 */
export function NotificationBell() {
  const [open, setOpen] = useState(false);
  const [attention, setAttention] = useState(0);
  const [items, setItems] = useState<AppNotification[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const { pathname, search } = useLocation();
  const isHost = hasRole(useAuth().user, "host");
  const { connected, subscribe } = useLive();
  const openRef = useRef(open);
  openRef.current = open;

  const refreshCount = useCallback(() => {
    fetchNotificationCount()
      .then((c) => setAttention(c.attention))
      .catch(() => {});
  }, []);

  // Shows what was unread when loaded, then marks it read on the server.
  const load = useCallback(async () => {
    const list = await fetchNotifications();
    setItems(list.notifications);
    setAttention(list.attention);
    if (list.notifications.some((n) => !n.read)) {
      await markNotificationsRead();
      refreshCount();
    }
  }, [refreshCount]);

  useEffect(refreshCount, [refreshCount]);

  // The socket says when this user's notifications change, and `bracket` when a match moves or
  // concludes, which can make a question no longer answerable (and after a reconnect). An open
  // panel shows what arrived and marks it read.
  useEffect(() => {
    let timer: number | undefined;
    const off = subscribe((event) => {
      if (event.type !== "notifications" && event.type !== "bracket") return;
      window.clearTimeout(timer);
      timer = window.setTimeout(() => {
        if (openRef.current) load().catch((e) => setError(errorMessage(e)));
        else refreshCount();
      }, RELOAD_DELAY_MS);
    });
    return () => {
      off();
      window.clearTimeout(timer);
    };
  }, [subscribe, load, refreshCount]);

  // Without the socket it polls, and looks again whenever the tab is shown.
  useEffect(() => {
    if (connected) return;
    const timer = setInterval(refreshCount, POLL_MS);
    const onVisible = () => {
      if (document.visibilityState === "visible") refreshCount();
    };
    document.addEventListener("visibilitychange", onVisible);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
    };
  }, [connected, refreshCount]);

  useEffect(() => {
    if (!open) return;
    setError(null);
    load().catch((e) => setError(errorMessage(e)));
  }, [open, load]);

  // Navigating anywhere else (the DevSwitcher, the back button) closes the panel too.
  useEffect(() => setOpen(false), [pathname, search]);

  useEffect(() => {
    if (!open) return;
    const onOutside = (e: PointerEvent) => {
      if (!rootRef.current?.contains(e.target as Node)) setOpen(false);
    };
    const onEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("pointerdown", onOutside);
    window.addEventListener("keydown", onEscape);
    return () => {
      window.removeEventListener("pointerdown", onOutside);
      window.removeEventListener("keydown", onEscape);
    };
  }, [open]);

  async function answer(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(errorMessage(e));
    }
    try {
      await load();
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }

  function decline(n: AppNotification) {
    const role = n.staff_role ? ROLE_AS[n.staff_role] : "";
    const others = n.staff_role === "referee" ? "hakemlere" : "yayıncılara";
    if (
      window.confirm(
        `Maçtan ${role} olarak çıkarılacaksın ve diğer ${others} ve hostlara haber verilecek. Emin misin?`,
      )
    ) {
      answer(() => declineNotification(n.id));
    }
  }

  return (
    <div className="notif" ref={rootRef}>
      <span className="notif-bell">
        <IconButton
          icon="bell"
          label={attention > 0 ? `Bildirimler (${attention})` : "Bildirimler"}
          variant="outlined"
          active={open}
          aria-expanded={open}
          onClick={() => setOpen((o) => !o)}
        />
        {attention > 0 && (
          <span className="notif-count" aria-hidden="true">
            {attention > 99 ? "99+" : attention}
          </span>
        )}
      </span>

      {open && (
        <div
          className="notif-panel"
          role="dialog"
          aria-label="Bildirimler"
          // Closes on a followed link even when it leads to the page already open.
          onClick={(e) => {
            if ((e.target as HTMLElement).closest("a")) setOpen(false);
          }}
        >
          <h2 className="notif-title">Bildirimler</h2>
          {error && <p className="alert alert-error notif-error">{error}</p>}
          {!items ? (
            <p className="status">Yükleniyor…</p>
          ) : items.length === 0 ? (
            <p className="status">Bildirim yok.</p>
          ) : (
            <ul className="notif-list">
              {items.map((n) => (
                <NotificationItem
                  key={n.id}
                  n={n}
                  isHost={isHost}
                  busy={busy}
                  onConfirm={() => answer(() => confirmNotification(n.id))}
                  onDecline={() => decline(n)}
                />
              ))}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}

const FOOT_LABELS: Record<string, string> = {
  "/team": "Takımım sayfasında gör",
  "/admin/teams": "Takımlar sayfasında gör",
  "/admin/users": "Kullanıcılar sayfasında gör",
  "/qualifiers": "Eleme lobilerini gör",
};

interface ItemProps {
  n: AppNotification;
  isHost: boolean;
  busy: boolean;
  onConfirm: () => void;
  onDecline: () => void;
}

function NotificationItem({ n, isHost, busy, onConfirm, onDecline }: ItemProps) {
  const role = n.staff_role ? ROLE_AS[n.staff_role] : "";
  const actor = n.actor_username ?? "Biri";
  const pending = n.kind === "confirm_time" && n.actionable;
  const classes = ["notif-item", !n.read && "is-unread", pending && "is-pending"]
    .filter(Boolean)
    .join(" ");
  // The player notifications lead to the team page; a host is sent to the teams admin instead.
  const teamPath =
    ((n.kind === "rank_over_limit" || n.kind === "discord_left") && isHost) ||
    n.kind === "player_banned"
      ? "/admin/teams"
      : "/team";
  const isBan = n.kind === "banned" || n.kind === "teammate_banned" || n.kind === "player_banned";
  // The auto referee's word to the staff, about a match they can open on the referee page.
  const fromAutoref =
    n.kind === "player_report" || n.kind === "emergency_stop" || n.kind === "autoref_stopped";
  const isDanger =
    isBan ||
    n.kind === "emergency_stop" ||
    n.kind === "autoref_stopped" ||
    n.kind === "no_show_forfeit" ||
    n.kind === "lobby_no_show";
  // Where the foot link goes: the team's page, or for a host told about a player with no team,
  // the users admin, where the ban is; a team taken out of its lobby picks another.
  const footPath =
    n.kind === "player_banned" && !n.team_name
      ? "/admin/users"
      : n.kind === "lobby_no_show"
        ? "/qualifiers"
        : teamPath;
  const forPlayers =
    n.kind === "reschedule_proposed" ||
    n.kind === "reschedule_accepted" ||
    n.kind === "rank_over_limit" ||
    n.kind === "discord_left" ||
    n.kind === "no_show_forfeit" ||
    n.kind === "lobby_no_show" ||
    isBan;
  const who = n.team_name ? `${actor} (${n.team_name})` : actor;
  const lobby = n.lobby_name ? `Lobi ${n.lobby_name}` : "eleme lobisi";
  // banned, player_banned: why, on a line of its own: before what the ban stops for the player,
  // after who it was for the hosts.
  let reason: string | null = null;

  let title: string;
  let body: string;
  switch (n.kind) {
    case "match_rescheduled":
      title = "Maç saati değişti";
      body = `Takımlar maçı yeni bir saate aldı; ${actor} kabul etti.`;
      break;
    case "confirm_time":
      title = "Yeni saati onayla";
      body = n.by_host
        ? `${actor} (host) ${role} olduğun maçı yeni bir saate aldı. Bu saatte müsait misin?`
        : `Takımlar ${role} olduğun maçı yeni bir saate aldı (${actor} kabul etti). Bu saatte müsait misin?`;
      break;
    case "staff_declined":
      title = n.staff_role === "referee" ? "Maçın hakemi yok" : "Maçın yayıncısı yok";
      body = `${actor} maçın yeni saatinde ${role} olamıyor ve maçtan çıktı. Alabilecek biri gerekiyor.`;
      break;
    case "reschedule_proposed":
      title = "Rakibin yeni saat önerdi";
      body =
        n.pending_proposals > 0
          ? `${actor} maç için yeni bir saat önerdi. Bekleyen ${n.pending_proposals} öneri var.`
          : `${actor} maç için yeni bir saat önerdi. Bekleyen öneri kalmadı.`;
      break;
    case "reschedule_accepted":
      title = "Maç saati değişti";
      body = `${actor} önerilen saati kabul etti, maç yeni saatine alındı.`;
      break;
    case "rank_over_limit":
      title = "Takım rank limitini aştı";
      body =
        `${actor} oyuncusunun rank'ı ${formatRank(n.old_rank)} → ${formatRank(n.new_rank)} oldu; takım değeri ` +
        `${formatTeamValue(n.team_value)} ile ${formatTeamValue(n.limit)} limitini aşıyor.${isHost ? "" : " Hostlar bilgilendirildi."}`;
      break;
    case "discord_left":
      title = "Oyuncu Discord'dan ayrıldı";
      body = `${actor} turnuvanın Discord sunucusundan ayrıldı. Maç kanallarını görebilmesi için sunucuya geri dönmesi gerekiyor.`;
      break;
    case "banned":
      title = "Turnuvadan banlandın";
      reason = n.ban_reason || null;
      body =
        "Takım kuramaz, takımını değiştiremez, eleme lobisine katılamaz ve maç saati öneremezsin.";
      break;
    case "player_banned":
      title = "Oyuncu banlandı";
      reason = n.ban_reason || null;
      body = n.team_name
        ? `${actor} turnuvadan banlandı. Takımı, bir host dağıtana kadar duruyor.`
        : `${actor} turnuvadan banlandı.`;
      break;
    case "teammate_banned":
      title = "Takım arkadaşın banlandı";
      body = `${actor} turnuvadan banlandı. Takımınız şimdilik duruyor; ne olacağına hostlar karar verecek.`;
      break;
    case "player_report":
      title = "Oyuncu raporu";
      reason = n.message;
      body = `${who} maç odasından hakemlere ve hostlara rapor gönderdi.`;
      break;
    case "emergency_stop":
      title = "Acil durdurma";
      body = `${who} maçı acil durumda durdurdu: maç manuel kontrole geçti, bir hakemin bakması gerekiyor.`;
      break;
    case "autoref_stopped":
      title = "Otomatik hakem durdu";
      reason = n.message;
      body = "Maç manuel kontrole geçti, bir hakemin devralması gerekiyor.";
      break;
    case "no_show_forfeit":
      title = "Maç hükmen bitti";
      body = `${n.team_name ?? "Bir takım"} maç saatinden sonraki 10 dakika içinde tam kadro gelmedi, maç hükmen sonuçlandı.`;
      break;
    case "lobby_no_show":
      title = "Eleme lobisinden çıkarıldınız";
      body = `Takımın ${lobby} için 10 dakika içinde tam kadro gelmediğinden lobiden çıkarıldı. Başlamamış başka bir lobiye kaydolabilirsin.`;
      break;
  }

  return (
    <li className={classes}>
      <div className="notif-head">
        <span className={isDanger ? "notif-item-title is-danger" : "notif-item-title"}>
          {title}
        </span>
        <span className="notif-when" title={formatDateTime(n.created_at)}>
          {formatShortMatchTime(n.created_at)}
        </span>
      </div>
      {fromAutoref && n.match_id ? (
        <Link to={`/admin/referee/${n.match_id}`} className="notif-match">
          {n.side && n.number !== null ? (
            <>
              <span className={`notif-code is-${n.side}`}>
                {matchCode({ side: n.side, number: n.number })}
              </span>
              <span className="notif-teams">
                {n.team1_name ?? "?"} <span className="muted">vs</span> {n.team2_name ?? "?"}
              </span>
            </>
          ) : (
            <span className="notif-teams">{lobby}</span>
          )}
        </Link>
      ) : n.match_id && n.side && n.number !== null ? (
        <Link
          to={forPlayers ? teamPath : bracketMatchPath(n.bracket_match_id ?? n.match_id, true)}
          className="notif-match"
        >
          <span className={`notif-code is-${n.side}`}>
            {matchCode({ side: n.side, number: n.number })}
          </span>
          {n.potential && <span className="badge badge-potential">Olası</span>}
          <span className="notif-teams">
            {n.team1_name ?? "?"} <span className="muted">vs</span> {n.team2_name ?? "?"}
          </span>
        </Link>
      ) : (
        n.team_name && (
          <Link to={teamPath} className="notif-match">
            <span className="notif-teams">{n.team_name}</span>
          </Link>
        )
      )}
      {n.new_starts_at && (
        <p className="notif-times">
          <span className="muted">
            {n.old_starts_at ? formatShortMatchTime(n.old_starts_at) : "Saat yoktu"}
          </span>
          {" → "}
          <strong>{formatShortMatchTime(n.new_starts_at)}</strong>
        </p>
      )}
      {reason && n.kind === "banned" && <p className="notif-reason">Sebep: {reason}</p>}
      {reason && n.kind === "player_report" && <p className="notif-reason">“{reason}”</p>}
      <p className="notif-body">{body}</p>
      {reason && n.kind === "player_banned" && <p className="notif-reason">Sebep: {reason}</p>}
      {reason && n.kind === "autoref_stopped" && <p className="notif-reason">Neden: {reason}</p>}

      {forPlayers && (
        <Link to={footPath} className="notif-link">
          {FOOT_LABELS[footPath]} →
        </Link>
      )}
      {n.kind === "confirm_time" &&
        (n.response ? (
          <span
            className={n.response === "confirmed" ? "badge badge-success" : "badge badge-danger"}
          >
            {n.response === "confirmed" ? "Onayladın" : "Reddettin, maçtan çıkarıldın"}
          </span>
        ) : n.actionable ? (
          <div className="notif-actions">
            <button
              type="button"
              className="btn btn-primary btn-sm"
              disabled={busy}
              onClick={onConfirm}
            >
              Onayla
            </button>
            <button
              type="button"
              className="btn btn-danger btn-sm"
              disabled={busy}
              onClick={onDecline}
            >
              Reddet
            </button>
          </div>
        ) : (
          <span className="muted notif-stale">
            Artık geçerli değil: maç değişti ya da artık bu maçta {role} değilsin.
          </span>
        ))}
    </li>
  );
}
