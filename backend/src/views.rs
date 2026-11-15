//! Response shapes several handlers share: a team with its players, a match's referee,
//! streamer and rooms, and a lobby's letter. The Discord bot, the room bots and the
//! statistics name lobbies the same way.

use serde::Serialize;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{
    db::{
        matches::RefereeColumns,
        team::{Player, TeamMemberRow},
    },
    team_rank,
};

// ---------- teams ----------

// Set on a formed team whose ranks no longer fit. Hosts decide what happens next.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RankWarning {
    OverLimit,
    Unranked,
}

#[derive(Debug, Serialize)]
pub struct Team<P> {
    pub id: Uuid,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    // Versioned by upload time, so the image can be cached forever.
    banner_url: Option<String>,
    pub team_value: Option<f64>,
    rank_warning: Option<RankWarning>,
    pub players: Vec<P>,
}

#[derive(Debug, Serialize)]
pub struct AdminTeamPlayer {
    #[serde(flatten)]
    player: Player,
    // For copying a mention (<@id>) that pings them on Discord.
    discord_id: Option<String>,
    discord_username: Option<String>,
    // False once they've left the tournament's Discord server.
    discord_in_guild: Option<bool>,
    // Banned by the hosts; the team stays until a host disbands it.
    banned: bool,
}

impl AdminTeamPlayer {
    pub fn from_row(row: TeamMemberRow) -> Self {
        Self {
            player: row.player,
            discord_id: row.discord_id,
            discord_username: row.discord_username,
            discord_in_guild: row.discord_in_guild,
            banned: row.banned,
        }
    }

    pub fn banned(&self) -> bool {
        self.banned
    }
}

// Rows arrive ordered by team, so consecutive rows with the same team_id belong together.
pub fn group_teams<P>(
    rows: Vec<TeamMemberRow>,
    to_player: impl Fn(TeamMemberRow) -> P,
) -> Vec<Team<P>> {
    let mut teams: Vec<(Team<P>, Vec<Player>)> = Vec::new();
    for row in rows {
        if teams.last().map(|(t, _)| t.id) != Some(row.team_id) {
            let team = Team {
                id: row.team_id,
                name: row.team_name.clone(),
                created_at: row.team_created_at,
                banner_url: row.banner_updated_at.map(|at| {
                    let version = at.unix_timestamp_nanos() / 1_000_000;
                    format!("/api/public/teams/{}/banner?v={version}", row.team_id)
                }),
                team_value: None,
                rank_warning: None,
                players: Vec::new(),
            };
            teams.push((team, Vec::new()));
        }
        let (team, players) = teams.last_mut().unwrap();
        players.push(row.player.clone());
        team.players.push(to_player(row));
    }
    teams
        .into_iter()
        .map(|(mut team, players)| {
            if let [a, b] = &players[..] {
                team.team_value = a
                    .global_rank
                    .zip(b.global_rank)
                    .map(|(a, b)| team_rank::team_value(a, b));
            }
            team.rank_warning = match team.team_value {
                None => Some(RankWarning::Unranked),
                Some(value) if value > team_rank::LIMIT => Some(RankWarning::OverLimit),
                Some(_) => None,
            };
            team
        })
        .collect()
}

// ---------- a match's staff and rooms ----------

#[derive(Debug, Serialize)]
// Public on purpose: players need to reach their match's referee on Discord.
pub struct Referee {
    id: Uuid,
    osu_id: i64,
    username: String,
    avatar_url: String,
    discord_id: Option<String>,
    discord_username: Option<String>,
}

impl Referee {
    pub fn from_columns(cols: RefereeColumns) -> Option<Self> {
        match (
            cols.referee_id,
            cols.referee_osu_id,
            cols.referee_username,
            cols.referee_avatar_url,
        ) {
            (Some(id), Some(osu_id), Some(username), Some(avatar_url)) => Some(Referee {
                id,
                osu_id,
                username,
                avatar_url,
                discord_id: cols.referee_discord_id,
                discord_username: cols.referee_discord_username,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Streamer {
    id: Uuid,
    osu_id: i64,
    username: String,
    avatar_url: String,
}

impl Streamer {
    // From the streamer_* columns, all null when nobody streams the match.
    pub fn from_columns(
        id: Option<Uuid>,
        osu_id: Option<i64>,
        username: Option<String>,
        avatar_url: Option<String>,
    ) -> Option<Self> {
        match (id, osu_id, username, avatar_url) {
            (Some(id), Some(osu_id), Some(username), Some(avatar_url)) => Some(Streamer {
                id,
                osu_id,
                username,
                avatar_url,
            }),
            _ => None,
        }
    }
}

// One of a match's osu! multiplayer rooms. Usually a match has one; a room that
// closes or breaks mid-match is replaced, and the old one stays listed.
#[derive(Debug, Serialize)]
pub struct MatchRoom {
    mp_id: i64,
    url: String,
}

pub fn match_rooms(mp_ids: Vec<i64>) -> Vec<MatchRoom> {
    mp_ids
        .into_iter()
        .map(|mp_id| MatchRoom {
            mp_id,
            url: format!("https://osu.ppy.sh/community/matches/{mp_id}"),
        })
        .collect()
}

// ---------- lobbies ----------

// A lobby's letter from its index: 1 is "A", 26 "Z", 27 "AA".
pub fn lobby_name(index: i32) -> String {
    let mut n = index.max(1) as u32;
    let mut name = Vec::new();
    while n > 0 {
        n -= 1;
        name.push(char::from(b'A' + (n % 26) as u8));
        n /= 26;
    }
    name.into_iter().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::lobby_name;

    #[test]
    fn lobby_names_run_through_the_alphabet() {
        assert_eq!(lobby_name(1), "A");
        assert_eq!(lobby_name(2), "B");
        assert_eq!(lobby_name(26), "Z");
        assert_eq!(lobby_name(27), "AA");
        assert_eq!(lobby_name(52), "AZ");
        assert_eq!(lobby_name(53), "BA");
        assert_eq!(lobby_name(702), "ZZ");
        assert_eq!(lobby_name(703), "AAA");
    }
}
