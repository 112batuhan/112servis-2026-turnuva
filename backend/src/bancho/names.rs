//! osu! usernames as Bancho writes them: compared without case, with IRC's underscores
//! for spaces. Used wherever a line's sender is matched to a player or a member of staff.

// Usernames compare without case, and IRC writes spaces as underscores.
pub fn normalize_name(name: &str) -> String {
    name.trim().replace('_', " ").to_lowercase()
}

pub fn same_name(a: &str, b: &str) -> bool {
    normalize_name(a) == normalize_name(b)
}

// The name to log in or address someone with on IRC.
pub fn irc_name(username: &str) -> String {
    username.trim().replace(' ', "_")
}
