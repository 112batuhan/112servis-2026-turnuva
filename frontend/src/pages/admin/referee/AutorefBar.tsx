import { useMemo } from "react";
import type { AutorefView } from "../../../api/types";
import { useNow } from "../../../components/match/MapClock";
import { Switch } from "../../../components/Switch";
import { formatClock } from "../../../lib/format";
import { matchTeams, poolEntries } from "../../../lib/matchPool";
import type { RefereeRoomState } from "./useRefereeRoom";
import "./AutorefBar.css";

/** How long before its time the bot opens a match's room (backend: OPEN_BEFORE). */
const OPENS_BEFORE_MS = 10 * 60 * 1000;
/** -abort uses each team has (backend: autoref::ABORTS). */
const ABORTS = 2;

/**
 * The switch in the room bar between the auto referee and manual control. Off (by a referee, or a
 * player's -durdur), the bot only answers -rapor, -durdur and -davet; back on, it continues where
 * it left off. The old controls work either way.
 */
export function AutorefSwitch({ referee }: { referee: RefereeRoomState }) {
  const { room, canRun, autoEnabled, actions } = referee;
  if (!room) return null;
  const title = !canRun
    ? "Bu maçın hakemi değilsin"
    : autoEnabled
      ? "Bot maçı kendisi yönetiyor. Kapatınca manuel kontrole geçer; açınca kaldığı yerden devam eder."
      : "Manuel kontrol. Açınca otomatik hakem kaldığı yerden devam eder.";
  return (
    <Switch
      checked={autoEnabled}
      disabled={!canRun || room.concluded_at !== null}
      onChange={actions.setAutoref}
      title={title}
    >
      Otomatik hakem
    </Switch>
  );
}

/**
 * Where the auto referee stands, one line under the room bar: what it's waiting for and until when,
 * then each team's -abort uses. Switched to manual, the line says so and what it picks up again;
 * before the room, whether the bot will open it.
 */
export function AutorefStatus({ referee }: { referee: RefereeRoomState }) {
  const { room, autoref, autoEnabled } = referee;
  const view = autoref;
  const deadline = view?.deadline ?? null;
  const now = useNow(deadline !== null && autoEnabled);
  const entries = useMemo(() => (room ? poolEntries(room) : []), [room]);
  if (!room || room.concluded_at) return null;

  // Before the room: when the bot will open it, unless the hosts have that switched off.
  if (room.rooms.length === 0) {
    if (!autoEnabled || !room.starts_at) return null;
    if (!room.auto_rooms) {
      return (
        <p className="autoref-status is-manual">
          <span className="autoref-dot" aria-hidden="true" />
          <span className="autoref-note">
            Otomatik oda açma kapalı (Bancho botları sayfası): odayı bir hakem açınca otomatik hakem
            maçı yönetir.
          </span>
        </p>
      );
    }
    const opensAt = new Date(room.starts_at).getTime() - OPENS_BEFORE_MS;
    return (
      <p className="autoref-status is-running">
        <span className="autoref-dot" aria-hidden="true" />
        <span className="autoref-phase">Otomatik hakem odayı açıp oyuncuları davet edecek</span>
        <span className="autoref-clock num" title="Odanın açılacağı saat">
          {formatClock(opensAt)}
        </span>
      </p>
    );
  }
  if (!view) return null;

  const teams = room.kind === "bracket" ? matchTeams(room) : [];
  const teamName = (id: string | null) => room.teams.find((t) => t.id === id)?.name ?? "Bir takım";
  const teamClass = (id: string | null) => {
    const color = teams.find((t) => t.id === id)?.color;
    return color ? `is-team-${color}` : undefined;
  };
  const label = entries.find((e) => e.map.id === view.map_id)?.label ?? "Map";
  const phase = phaseText(view, room.starts_at, now, teamName, teamClass, label);

  if (!autoEnabled) {
    return (
      <p className="autoref-status is-manual">
        <span className="autoref-dot" aria-hidden="true" />
        <strong>Manuel kontrol</strong>
        <span className="autoref-note">
          Bot sadece -rapor, -durdur ve -davet'e cevap veriyor. Açınca devam eder: {phase.plain}
        </span>
      </p>
    );
  }

  const left = deadline ? new Date(deadline).getTime() - now : null;
  return (
    <div className="autoref-status is-running" role="status">
      <span className="autoref-dot" aria-hidden="true" />
      <span className="autoref-phase">{phase.node}</span>
      {left !== null && left > 0 && (
        <span className="autoref-clock num" title={phase.clockTitle}>
          {countdown(left)}
        </span>
      )}
      {teams.length === 2 && (
        <span className="autoref-uses" aria-label="Takımların abort hakları">
          {teams.map((t) => {
            const aborts = view.teams.find((u) => u.team_id === t.id)?.aborts_left ?? ABORTS;
            return (
              <span key={t.id} className="autoref-team">
                <span className={`autoref-team-name is-team-${t.color}`}>{t.name}</span>
                <span
                  className={`autoref-use ${aborts === 0 ? "is-spent" : ""}`}
                  title={`-abort: ${aborts}/${ABORTS} hakkı kaldı`}
                >
                  abort <span className="num">{aborts}</span>
                </span>
              </span>
            );
          })}
        </span>
      )}
    </div>
  );
}

function countdown(ms: number): string {
  const total = Math.ceil(ms / 1000);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, "0")}`;
}

/** The phase in words: as a node (team names in their colour) and as plain text. */
function phaseText(
  view: AutorefView,
  startsAt: string | null,
  now: number,
  teamName: (id: string | null) => string,
  teamClass: (id: string | null) => string | undefined,
  label: string,
) {
  const team = <strong className={teamClass(view.team_id)}>{teamName(view.team_id)}</strong>;
  const name = teamName(view.team_id);
  const before = startsAt !== null && now < new Date(startsAt).getTime();
  switch (view.phase) {
    case "waiting":
      return before
        ? {
            node: <>Maç saati bekleniyor, herkes hazırsa erken başlar</>,
            plain: "maç saati bekleniyor",
            clockTitle: "Maç saatine",
          }
        : {
            node: <>Oyuncular bekleniyor; gelmeyen takım elenir</>,
            plain: "oyuncular bekleniyor",
            clockTitle: "Gelmeyen takımın elenmesine",
          };
    case "warmup":
      return {
        node: view.warmup_playing ? (
          <>Warmup: {team} oynuyor</>
        ) : view.warmup_beatmap_id !== null ? (
          <>Warmup: {team} seçti, hazır olunca başlar</>
        ) : (
          <>Warmup: {team} maç odası sayfasından seçiyor</>
        ),
        plain: `${name} warmup'ı`,
        clockTitle: "Warmup süresi",
      };
    case "roll":
      return { node: <>!roll bekleniyor</>, plain: "roll", clockTitle: "Roll süresi" };
    case "draft": {
      const step = view.step === "ban" ? "ban" : "pick";
      return {
        node: (
          <>
            {team} {step} sırası, maç odası sayfasından seçmeleri bekleniyor
          </>
        ),
        plain: `${name} ${step} sırası`,
        clockTitle: "",
      };
    }
    case "ready":
      return {
        node: (
          <>
            <strong className="num">{label}</strong> açık, herkesin hazır olması bekleniyor
          </>
        ),
        plain: `${label} hazırlığı`,
        clockTitle: "Sayaç sıfırlanınca başlar",
      };
    case "playing":
      return {
        node: (
          <>
            <strong className="num">{label}</strong> oynanıyor
          </>
        ),
        plain: `${label} oynanıyor`,
        clockTitle: "",
      };
    case "scoring":
      return {
        node: (
          <>
            <strong className="num">{label}</strong> skorları osu!'dan okunuyor
          </>
        ),
        plain: "skorlar",
        clockTitle: "",
      };
    case "next":
      return { node: <>Sıradaki adım hazırlanıyor</>, plain: "sıradaki adım", clockTitle: "" };
    case "done":
      return { node: <>Maç bitti</>, plain: "maç bitti", clockTitle: "" };
  }
}
