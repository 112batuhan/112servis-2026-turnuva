import type { Player } from "../api/types";
import { formatRank } from "../lib/format";
import "./PlayerChip.css";

export function PlayerChip({ player }: { player: Player }) {
  return (
    <div className="player-chip">
      {player.avatar_url && <img className="player-chip-avatar" src={player.avatar_url} alt="" loading="lazy" />}
      <div className="player-chip-text">
        <a
          className="player-chip-name"
          href={`https://osu.ppy.sh/users/${player.osu_id}`}
          target="_blank"
          rel="noopener noreferrer"
        >
          {player.username}
        </a>
        <span className="player-chip-meta">
          {formatRank(player.global_rank)}
          {player.country_code && <> · {player.country_code}</>}
        </span>
      </div>
    </div>
  );
}
