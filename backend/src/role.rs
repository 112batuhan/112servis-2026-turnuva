use serde::{Deserialize, Serialize};

// A staff role. A user holds any set of them (`osu_users.roles`, a `staff_role[]`); none
// means a regular player. Loaded with the session on every request (see
// auth::AuthUser).
//
// Roles don't imply each other, with one exception: Host is allowed everything the
// others are (`Roles::allows`). Acting in a role — being offered for, taking or being
// pinged about a referee or streamer slot — needs the role itself (`Roles::has`), so a
// host who referees ticks Referee too. Graphics designers and commentators have no
// authority at all (they're only listed on the staff page); pool testers and custom mappers can only
// look at the map pool editor (`Roles::can_view_pool`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, sqlx::Type,
)]
#[sqlx(type_name = "staff_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Host,
    MapPooler,
    Referee,
    Streamer,
    GraphicsDesigner,
    PoolTester,
    CustomMapper,
    Commentator,
}

// So a `staff_role[]` column reads into a Vec<Role>; sqlx 0.7 doesn't derive it.
impl sqlx::postgres::PgHasArrayType for Role {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        sqlx::postgres::PgTypeInfo::with_name("_staff_role")
    }
}

impl Role {
    fn bit(self) -> u8 {
        1 << self as u8
    }
}

// The roles one user holds, as a small set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Roles(u8);

impl Roles {
    pub fn of(roles: &[Role]) -> Self {
        Self(roles.iter().fold(0, |bits, r| bits | r.bit()))
    }

    // Holds the role itself.
    pub fn has(self, role: Role) -> bool {
        self.0 & role.bit() != 0
    }

    // May do what the role may: holds it, or is a host.
    pub fn allows(self, role: Role) -> bool {
        self.has(role) || self.has(Role::Host)
    }

    // Whether the user can be in a team. Streamers, graphics designers and commentators
    // are the only staff who also play.
    pub fn can_play(self) -> bool {
        let playing = Role::Streamer.bit() | Role::GraphicsDesigner.bit() | Role::Commentator.bit();
        self.0 & !playing == 0
    }

    // May open the map pool editor: map poolers (and hosts) to edit it, pool testers and
    // custom mappers only to look.
    pub fn can_view_pool(self) -> bool {
        self.allows(Role::MapPooler) || self.has(Role::PoolTester) || self.has(Role::CustomMapper)
    }
}

// A role list as stored: each role once, in a fixed order.
pub fn normalize(mut roles: Vec<Role>) -> Vec<Role> {
    roles.sort();
    roles.dedup();
    roles
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_allows_everything_but_holds_only_host() {
        let host = Roles::of(&[Role::Host]);
        assert!(host.allows(Role::Referee) && host.allows(Role::MapPooler));
        assert!(!host.has(Role::Referee));
        assert!(!host.can_play());
    }

    #[test]
    fn roles_dont_imply_each_other() {
        let pooler = Roles::of(&[Role::MapPooler]);
        assert!(pooler.allows(Role::MapPooler));
        assert!(!pooler.allows(Role::Referee) && !pooler.allows(Role::Streamer));
    }

    #[test]
    fn only_streamers_designers_commentators_and_players_play() {
        assert!(Roles::default().can_play());
        assert!(Roles::of(&[Role::Streamer]).can_play());
        assert!(Roles::of(&[Role::Streamer, Role::GraphicsDesigner]).can_play());
        assert!(Roles::of(&[Role::Commentator]).can_play());
        assert!(!Roles::of(&[Role::Streamer, Role::Referee]).can_play());
        assert!(!Roles::of(&[Role::PoolTester]).can_play());
        assert!(!Roles::of(&[Role::CustomMapper]).can_play());
    }

    #[test]
    fn pool_viewers_cant_edit() {
        for role in [Role::PoolTester, Role::CustomMapper] {
            let viewer = Roles::of(&[role]);
            assert!(viewer.can_view_pool());
            assert!(!viewer.allows(Role::MapPooler));
        }
        assert!(Roles::of(&[Role::Host]).can_view_pool());
        assert!(Roles::of(&[Role::MapPooler]).can_view_pool());
        assert!(!Roles::of(&[Role::GraphicsDesigner, Role::Streamer]).can_view_pool());
        assert!(!Roles::of(&[Role::Commentator]).can_view_pool());
    }
}
