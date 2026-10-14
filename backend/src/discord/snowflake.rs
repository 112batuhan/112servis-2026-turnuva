//! Discord ids are 64-bit; they go to the browser as strings, which a JavaScript number
//! couldn't hold exactly, and are stored as BIGINT.

use serenity::all::{ChannelId, RoleId, UserId};

pub fn channel(id: i64) -> ChannelId {
    ChannelId::new(id as u64)
}

pub fn role(id: i64) -> RoleId {
    RoleId::new(id as u64)
}

// A linked account's id, unless it isn't a real one (the dev seed's are "dummy-1").
pub fn user(id: &str) -> Option<UserId> {
    id.parse::<u64>().ok().filter(|&n| n > 0).map(UserId::new)
}

pub mod option {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(id: &Option<i64>, s: S) -> Result<S::Ok, S::Error> {
        match id {
            Some(id) => s.serialize_str(&id.to_string()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<i64>, D::Error> {
        match Option::<String>::deserialize(d)? {
            None => Ok(None),
            Some(s) if s.trim().is_empty() => Ok(None),
            Some(s) => s
                .trim()
                .parse::<i64>()
                .ok()
                .filter(|&n| n > 0)
                .map(Some)
                .ok_or_else(|| D::Error::custom("not a Discord id")),
        }
    }
}
