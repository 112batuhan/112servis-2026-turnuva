import { useContext, type CSSProperties, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { Link } from "react-router-dom";
import type { Team } from "../../api/types";
import { TeamBanner } from "../../components/bracket/TeamBanner";
import { LiveMark } from "../../components/LiveMark";
import { TeamPopover } from "../../components/TeamPopover";
import { matchPagePath } from "../../lib/bracket";
import type { PoolEntry } from "../../lib/matchPool";
import { formatZ, ToolsSlot, type StatPlayer, type StatsIndex } from "./statsModel";
import "./StatsParts.css";

/** A section's controls (search, ordering, view), set at the end of the tab strip's row. */
export function PanelTools({ children }: { children: ReactNode }) {
  const slot = useContext(ToolsSlot);
  return slot ? createPortal(children, slot) : null;
}

// How many deviations fill half the meter.
const Z_SPAN = 3;

/**
 * A Z score as its figure beside a small meter centred on the map's mean: filled right of the
 * centre for a score above it, left for one below, in proportion up to three deviations.
 */
export function ZValue({ z, missing = false, size = "md" }: { z: number; missing?: boolean; size?: "md" | "lg" }) {
  const half = (Math.min(Math.abs(z), Z_SPAN) / Z_SPAN) * 50;
  const tone = missing ? "is-missing" : z >= 0 ? "is-up" : "is-down";
  return (
    <span className={`zval ${tone} is-${size}`}>
      <span className="zval-num">{formatZ(z)}</span>
      <span className="zval-bar" aria-hidden="true">
        <span className="zval-fill" style={{ left: z >= 0 ? "50%" : `${50 - half}%`, width: `${half}%` }} />
      </span>
    </span>
  );
}

/** A pool map's label ("NM1") on its category's colour. */
export function MapLabel({ entry }: { entry: PoolEntry | undefined }) {
  return (
    <span className="stat-map-label" style={{ "--category": entry?.color } as CSSProperties}>
      {entry?.label ?? "?"}
    </span>
  );
}

/** A player's avatar and osu! name, linking to their profile, with their team under it. */
export function PlayerName({
  player,
  index,
  own,
  showTeam = true,
}: {
  player: StatPlayer | undefined;
  index: StatsIndex;
  own: boolean;
  showTeam?: boolean;
}) {
  if (!player) return <span className="stat-who is-unknown">Bilinmeyen oyuncu</span>;
  const team = index.team.get(player.team_id);
  return (
    <span className={own ? "stat-who is-own" : "stat-who"}>
      <img className="stat-avatar" src={player.avatar_url} alt="" loading="lazy" />
      <span className="stat-who-text">
        <a
          className="stat-who-name"
          href={`https://osu.ppy.sh/users/${player.osu_id}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          {player.username}
        </a>
        {showTeam && team && <TeamName team={team} own={own} quiet />}
      </span>
    </span>
  );
}

/** A team's name that opens its card; `quiet` sets it as the small line under a player. */
export function TeamName({ team, own, quiet = false }: { team: Team; own: boolean; quiet?: boolean }) {
  return (
    <TeamPopover team={team} className={`stat-team-name${quiet ? " is-quiet" : ""}${own ? " is-own" : ""}`}>
      {team.name}
    </TeamPopover>
  );
}

/** A team's banner crop and name, with its players under it. */
export function TeamWho({ team, own }: { team: Team | undefined; own: boolean }) {
  if (!team) return <span className="stat-who is-unknown">Bilinmeyen takım</span>;
  return (
    <span className={own ? "stat-who is-team is-own" : "stat-who is-team"}>
      <TeamBanner team={team} />
      <span className="stat-who-text">
        <TeamName team={team} own={own} />
        <span className="stat-who-meta">{team.players.map((p) => p.username).join(" · ")}</span>
      </span>
    </span>
  );
}

/** The match a play was in, linking to its page. */
export function MatchCode({ matchId, index }: { matchId: string; index: StatsIndex }) {
  const m = index.match.get(matchId);
  if (!m) return null;
  const side = m.side === "winners" ? " is-winners" : m.side === "losers" ? " is-losers" : "";
  return (
    <Link className={`stat-match${side}`} to={matchPagePath(m.id)} title={`${m.code} maç sayfası`}>
      {m.code}
      {m.live && <LiveMark compact />}
    </Link>
  );
}
