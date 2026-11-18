//! What a pool category is (`stage_categories.kind`), and the rules each kind brings.
//! Mirrored in `frontend/src/lib/bancho.ts` (`CATEGORY_KINDS`, `isFreemodKind`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "category_kind", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    // NoMod: a team may pick from it twice in a row.
    Nomod,
    // A mod of its own (HD, HR, DT…).
    Modded,
    // The players choose their mods.
    Freemod,
    // Freemod too; nobody bans or picks it, it's played when the score is tied.
    Tiebreaker,
}

impl CategoryKind {
    // The players choose their own mods: FM and TB.
    pub fn freemod(self) -> bool {
        matches!(self, Self::Freemod | Self::Tiebreaker)
    }

    pub fn tiebreaker(self) -> bool {
        self == Self::Tiebreaker
    }

    // A team may pick from it twice in a row: NoMod only.
    pub fn repeatable(self) -> bool {
        self == Self::Nomod
    }
}

// A map's own mods may name its slot's kind instead of a mod: FM or TB, a map added for a
// freemod or tiebreaker slot. Played with Freemod, its stats the nomod ones.
pub const FREEMOD_MAP: &str = "FM";
pub const TIEBREAKER_MAP: &str = "TB";

// The mods a pool map is played with: its own, without the FM/TB mark.
pub fn playable_mods(mods: &str) -> &str {
    if is_kind_mark(mods) {
        ""
    } else {
        mods
    }
}

// The map was added as an FM or TB map.
pub fn is_kind_mark(mods: &str) -> bool {
    let mods = mods.trim();
    mods.eq_ignore_ascii_case(FREEMOD_MAP) || mods.eq_ignore_ascii_case(TIEBREAKER_MAP)
}

// A category's short name as it's stored: what was typed, upper-cased, when it's 1-6
// letters.
pub fn abbreviation(typed: &str) -> Option<String> {
    let abbr = typed.trim().to_uppercase();
    (!abbr.is_empty() && abbr.len() <= 6 && abbr.bytes().all(|b| b.is_ascii_uppercase()))
        .then_some(abbr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kinds_bring_their_rules() {
        assert!(CategoryKind::Freemod.freemod() && CategoryKind::Tiebreaker.freemod());
        assert!(!CategoryKind::Modded.freemod() && !CategoryKind::Nomod.freemod());
        assert!(CategoryKind::Tiebreaker.tiebreaker() && !CategoryKind::Freemod.tiebreaker());
        assert!(CategoryKind::Nomod.repeatable() && !CategoryKind::Modded.repeatable());
    }

    #[test]
    fn fm_and_tb_maps_play_without_mods() {
        assert_eq!(playable_mods("FM"), "");
        assert_eq!(playable_mods("tb"), "");
        assert_eq!(playable_mods("HDDT"), "HDDT");
        assert!(is_kind_mark("FM") && !is_kind_mark("HD"));
    }

    #[test]
    fn abbreviations_are_letters() {
        assert_eq!(abbreviation(" nm ").as_deref(), Some("NM"));
        assert_eq!(abbreviation("HDHR").as_deref(), Some("HDHR"));
        assert_eq!(abbreviation(""), None);
        assert_eq!(abbreviation("NM1"), None);
        assert_eq!(abbreviation("TOOLONG"), None);
        assert_eq!(abbreviation("ş"), None);
    }
}
