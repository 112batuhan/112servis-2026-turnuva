use serde::{Deserialize, Serialize};

// User authorization role. Stored in Postgres as the `user_role` enum and loaded
// with the session on every request (see auth::AuthUser).
//
// For now privileges are a simple ladder: Host can do everything, MapPooler can do
// map-pool work plus anything a Referee can, a Referee runs matches, a Streamer signs up
// to broadcast bracket matches, and Basic is the default for everyone else.
// If the model outgrows a linear hierarchy, replace `rank`/`has_at_least` with a
// real permission set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Host,
    MapPooler,
    Referee,
    Streamer,
    Basic,
}

impl Role {
    fn rank(self) -> u8 {
        match self {
            Role::Basic => 0,
            Role::Streamer => 1,
            Role::Referee => 2,
            Role::MapPooler => 3,
            Role::Host => 4,
        }
    }

    // True if this role is at least as privileged as `required`.
    pub fn has_at_least(self, required: Role) -> bool {
        self.rank() >= required.rank()
    }

    // Whether this role can be in a team. Streamers are the only staff who also play.
    pub fn can_play(self) -> bool {
        matches!(self, Role::Basic | Role::Streamer)
    }
}
