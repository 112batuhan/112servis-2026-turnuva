//! What a pool category's name says about its maps. Each name starts with its mod (NM1,
//! HD, FM2, TB), whatever its case or surrounding spaces, and so does every pool label
//! made from it ("FM1"). Mirrored in `frontend/src/lib/bancho.ts` (`isFreemodCategory`)
//! and `lib/matchPool.ts` (the tiebreaker).

fn starts_with(name: &str, prefix: &str) -> bool {
    name.trim().to_ascii_uppercase().starts_with(prefix)
}

// Freemod (FM) and the tiebreaker (TB): the players choose their own mods.
pub fn is_freemod(name: &str) -> bool {
    starts_with(name, "FM") || starts_with(name, "TB")
}

pub fn is_tiebreaker(name: &str) -> bool {
    starts_with(name, "TB")
}

// A team may pick from it twice in a row: NoMod only.
pub fn repeatable(name: &str) -> bool {
    starts_with(name, "NM")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_mod_from_the_name() {
        assert!(is_freemod("FM2") && is_freemod(" tb ") && !is_freemod("HD1"));
        assert!(is_tiebreaker("TB") && !is_tiebreaker("FM1"));
        assert!(repeatable("NM1") && repeatable("nm") && !repeatable("HD2"));
    }
}
