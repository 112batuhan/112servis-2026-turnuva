import { useState, type FormEvent, type ReactNode } from "react";
import type { AutorefView } from "../../../../api/types";
import { formatMatchTime } from "../../../../lib/format";
import { stepLabel } from "../../../../lib/draftOrder";
import type { MatchPageState } from "../../../../lib/useMatchPage";
import { clock, parseBeatmapId, shownEnd } from "../../../../lib/playerRoom";
import type { RoomMe } from "./PlayerRoomPage";
import "./TurnBar.css";

interface Props {
  page: MatchPageState;
  me: RoomMe;
  now: number;
  busy: boolean;
  onWarmup: (beatmapId: number) => void;
  onSkipWarmup: () => void;
}

/** Whose the moment is: the viewer's team (frost), a side (its colour), or nobody's. */
type Tone = "yours" | "blue" | "red" | "neutral" | "done";

interface Turn {
  tone: Tone;
  title: ReactNode;
  detail?: ReactNode;
  /** When the clock the room was told runs out, ms. */
  end: number | null;
  /** What the clock counts: "kaldı" for a turn, "başlamasına" before the match. */
  unit?: string;
  extra?: ReactNode;
}

/**
 * The top of the room page: what the match waits for right now and whose clock is running, at every
 * moment, from the room's bot (`LiveRoom.autoref`). Frost when it's the viewer's team's turn. The
 * team's warmup is chosen here, and a roll's figures show as they come.
 */
export function TurnBar({ page, me, now, busy, onWarmup, onSkipWarmup }: Props) {
  const turn = describe(page, me, now, busy, onWarmup, onSkipWarmup);
  const left = turn.end !== null ? turn.end - now : null;
  const urgent = turn.tone === "yours" && left !== null && left <= 30_000;
  return (
    <section
      className={`turn is-${turn.tone}${urgent ? " is-urgent" : ""}`}
      aria-label="Maçın durumu"
    >
      <div className="turn-main" aria-live="polite">
        <h2 className="turn-title">{turn.title}</h2>
        {turn.detail && <p className="turn-detail">{turn.detail}</p>}
      </div>
      {left !== null && left > -1000 && (
        <div className="turn-clock">
          <span className="turn-clock-figure num" role="timer" aria-label={`${clock(left)} kaldı`}>
            {clock(left)}
          </span>
          <span className="turn-clock-unit">{turn.unit ?? "kaldı"}</span>
        </div>
      )}
      {turn.extra && <div className="turn-extra">{turn.extra}</div>}
    </section>
  );
}

function describe(
  page: MatchPageState,
  me: RoomMe,
  now: number,
  busy: boolean,
  onWarmup: (id: number) => void,
  onSkipWarmup: () => void,
): Turn {
  const { match, lobby, roster, currentEntry, teams } = page;
  if (!match) return { tone: "neutral", title: "Yükleniyor…", end: null };
  const autoref: AutorefView | null = match.live?.autoref ?? null;
  const bracket = match.kind === "bracket";
  const teamName = (id: string | null) => match.teams.find((t) => t.id === id)?.name ?? "?";
  const sideTone = (id: string | null): Tone => {
    if (id === me.teamId) return "yours";
    const side = teams.find((t) => t.id === id)?.color;
    return side ?? "neutral";
  };
  // A team's name in its side's colour.
  const team = (id: string | null) => {
    const side = teams.find((t) => t.id === id)?.color;
    return <span className={side ? `turn-team is-team-${side}` : "turn-team"}>{teamName(id)}</span>;
  };

  if (match.concluded_at) {
    return {
      tone: "done",
      title: match.forfeit_slot ? "Maç hükmen bitti" : "Maç bitti",
      detail: "Skor ve oynanan mapler sağda.",
      end: null,
    };
  }

  const startsAt = match.starts_at;
  if (match.rooms.length === 0) {
    const future = startsAt !== null && new Date(startsAt).getTime() > now;
    return {
      tone: "neutral",
      title: "Oda henüz açılmadı",
      detail: future ? (
        <>
          Maç <time dateTime={startsAt}>{formatMatchTime(startsAt)}</time>. Oda maç saatinden önce
          açılır, davet osu!'da gelir.
        </>
      ) : (
        "Oda açılınca davet osu!'da gelir; bu sayfa kendiliğinden güncellenir."
      ),
      end: future && startsAt ? new Date(startsAt).getTime() : null,
      unit: "başlamasına",
    };
  }

  if (!autoref) {
    return {
      tone: "neutral",
      title: "Odanın botu bağlanıyor",
      detail: "Bot odaya dönünce maç kaldığı yerden devam eder.",
      end: null,
    };
  }

  if (!autoref.enabled) {
    return {
      tone: "neutral",
      title: "Maçı şu an hakem yönetiyor",
      detail:
        "Ban, pick ve warmup'ları hakem odada alır. Rapor ve durdur çalışmaya devam eder; otomatik hakem dönünce sayfa yine buradan yürür.",
      end: null,
    };
  }

  const end = shownEnd(autoref.deadline, startsAt);
  const badges = match.live?.rooms ?? [];
  const bot = badges.length > 0 ? badges[badges.length - 1].bot : null;

  switch (autoref.phase) {
    case "waiting": {
      const missing = roster.filter((r) => !page.lobby.slots.some((s) => page.findRoster(s) === r));
      const future = startsAt !== null && new Date(startsAt).getTime() > now;
      return {
        tone: "neutral",
        title: future ? "Maç saati bekleniyor" : "Eksik oyuncular bekleniyor",
        detail: (
          <>
            {missing.length > 0 ? (
              <>
                Odada değil:{" "}
                <span className="turn-names">
                  {missing.map((r) => r.player.username).join(", ")}
                </span>
                .{" "}
              </>
            ) : (
              "Herkes odada; herkes hazır olunca maç erken başlar. "
            )}
            {!future &&
              (bracket
                ? "Süre dolunca tam kadro gelmeyen takım hükmen kaybeder."
                : "Süre dolunca tam kadro gelmeyen takımlar lobiden çıkarılır.")}
            {bot && missing.some((r) => r.player.id === me.userId) && (
              <> Davet gelmediyse osu!'da {bot} hesabına özelden -davet yaz.</>
            )}
          </>
        ),
        end,
        unit: future ? "başlamasına" : "kaldı",
      };
    }

    case "warmup": {
      const mine = autoref.team_id === me.teamId;
      const order = match.teams.findIndex((t) => t.id === autoref.team_id);
      const myOrder = match.teams.findIndex((t) => t.id === me.teamId);
      const skipped = autoref.warmup_skipped.includes(me.teamId);
      const chosen = autoref.warmup_beatmap_id;
      const onRoom = chosen !== null && lobby.beatmapId === chosen ? lobby.beatmapName : null;
      if (autoref.warmup_playing) {
        return {
          tone: sideTone(autoref.team_id),
          title: <>{team(autoref.team_id)} warmup'ı oynanıyor</>,
          detail: onRoom ?? lobby.beatmapName ?? undefined,
          end: null,
        };
      }
      return {
        tone: sideTone(autoref.team_id),
        title: mine ? "Warmup sırası sizde" : <>{team(autoref.team_id)} warmup seçiyor</>,
        detail: chosen ? (
          <>
            Odada: <span className="turn-names">{onRoom ?? `#${chosen}`}</span>. Herkes hazır olunca
            ya da süre dolunca başlar.
          </>
        ) : mine ? (
          "osu! linkini ya da beatmap ID'sini yapıştır; bot odaya koyar. Warmup 4 dakikayı geçmemeli, havuzdan bir map olamaz."
        ) : myOrder > order ? (
          skipped ? (
            "Sıranız gelince warmup'ınız geçilecek."
          ) : (
            "Sıra size gelince 3 dakikanız olacak."
          )
        ) : undefined,
        end,
        extra: mine ? (
          <WarmupForm chosen={chosen} busy={busy} onWarmup={onWarmup} onSkip={onSkipWarmup} />
        ) : myOrder > order && !skipped ? (
          <button
            type="button"
            className="btn btn-secondary btn-sm"
            disabled={busy}
            onClick={onSkipWarmup}
          >
            Sıramız gelince geç
          </button>
        ) : undefined,
      };
    }

    case "roll":
      return {
        tone: "neutral",
        title: "Roll",
        detail: "osu!'da !roll yazın; her takımın ilk roll'u sayılır. Kazanan ilk adımı alır.",
        end,
        extra: (
          <ul className="turn-rolls" aria-label="Roll'lar">
            {teams.map((t) => {
              const roll = autoref.rolls.find((r) => r.team_id === t.id)?.roll;
              return (
                <li key={t.id} className={`turn-roll is-team-${t.color}`}>
                  <span className="turn-roll-team">{t.name}</span>
                  <span className="turn-roll-figure num">{roll ?? "–"}</span>
                </li>
              );
            })}
          </ul>
        ),
      };

    case "draft": {
      const mine = autoref.team_id === me.teamId;
      const kind = autoref.step ?? "pick";
      return {
        tone: sideTone(autoref.team_id),
        title: mine ? (
          `${stepLabel(kind)} sırası sizde`
        ) : (
          <>
            {team(autoref.team_id)} {kind === "ban" ? "ban" : "pick"} seçiyor
          </>
        ),
        detail: mine
          ? kind === "ban"
            ? "Banlamak istediğin mapin üstüne 3 saniye basılı tut. Süre dolarsa ban hakkınız yanar."
            : "Seçmek istediğin mapin üstüne 3 saniye basılı tut. Süre dolarsa pick sırası rakibe geçer."
          : "Rakibin seçimi buraya ve havuza anında düşer.",
        end,
      };
    }

    case "ready": {
      const ready = lobby.slots.filter(
        (s) => s.status === "ready" && page.findRoster(s) !== undefined,
      ).length;
      return {
        tone: "neutral",
        title: (
          <>
            <MapLabel page={page} /> açıldı
          </>
        ),
        detail: (
          <>
            <span className="num">
              {ready}/{roster.length}
            </span>{" "}
            hazır. Herkes hazır olunca ya da süre dolunca başlar.
            {currentEntry?.freemod && " Freemod: NoFail'i unutma."}
          </>
        ),
        end,
        unit: "başlamasına",
      };
    }

    case "playing": {
      const length = currentEntry?.map.total_length;
      const playEnd =
        page.mapStartedAt !== null && length ? page.mapStartedAt + length * 1000 : null;
      return {
        tone: "neutral",
        title: (
          <>
            <MapLabel page={page} /> oynanıyor
          </>
        ),
        detail: lobby.beatmapName ?? undefined,
        end: playEnd,
      };
    }

    case "scoring":
      return {
        tone: "neutral",
        title: "Skorlar okunuyor",
        detail: "osu!'nun kaydı okununca sıradaki adım gelir.",
        end: null,
      };

    case "next":
      return { tone: "neutral", title: "Sıradaki adım hazırlanıyor", end: null };

    case "done":
      return { tone: "done", title: "Maç bitti", end: null };
  }
}

function MapLabel({ page }: { page: MatchPageState }) {
  const entry = page.currentEntry;
  if (!entry) return <>Map</>;
  return (
    <span className="turn-map" style={{ color: entry.color }}>
      {entry.label}
    </span>
  );
}

/** The team's warmup: a link or id, put on by the bot; or none at all. */
function WarmupForm({
  chosen,
  busy,
  onWarmup,
  onSkip,
}: {
  chosen: number | null;
  busy: boolean;
  onWarmup: (id: number) => void;
  onSkip: () => void;
}) {
  const [text, setText] = useState("");
  const id = parseBeatmapId(text);
  const submit = (e: FormEvent) => {
    e.preventDefault();
    if (id !== null) onWarmup(id);
  };
  return (
    <form className="turn-warmup" onSubmit={submit}>
      <input
        className="input turn-warmup-input"
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder="osu.ppy.sh/beatmapsets/…#osu/… ya da ID"
        aria-label="Warmup beatmap'inin linki ya da ID'si"
        aria-invalid={text.trim() !== "" && id === null}
        spellCheck={false}
        autoComplete="off"
      />
      <button type="submit" className="btn btn-primary" disabled={busy || id === null}>
        {chosen ? "Değiştir" : "Odaya koy"}
      </button>
      <button type="button" className="btn btn-secondary" disabled={busy} onClick={onSkip}>
        Warmup oynamayacağız
      </button>
      {text.trim() !== "" && id === null && (
        <p className="turn-warmup-hint">
          Bu bir beatmap linki değil. Set linkinde zorluk da seçili olmalı (…#osu/123).
        </p>
      )}
    </form>
  );
}
