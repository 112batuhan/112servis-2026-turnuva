use serde::{Deserialize, Serialize};

// User authorization role. Stored in Postgres as the `user_role` enum and carried
// in the JWT so most endpoints can gate on it without a database round-trip.
//
// For now privileges are a simple ladder: Host can do everything, MapPooler can do
// map-pool work plus anything Basic can, Basic is the default for everyone else.
// If the model outgrows a linear hierarchy, replace `rank`/`has_at_least` with a
// real permission set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Host,
    MapPooler,
    Basic,
}

impl Role {
    fn rank(self) -> u8 {
        match self {
            Role::Basic => 0,
            Role::MapPooler => 1,
            Role::Host => 2,
        }
    }

    // True if this role is at least as privileged as `required`.
    pub fn has_at_least(self, required: Role) -> bool {
        self.rank() >= required.rank()
    }
}
