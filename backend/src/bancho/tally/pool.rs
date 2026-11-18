//! The match's pool maps, and which of them a played beatmap was.

use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    bancho::room::split_mods,
    category,
    db::mappool::{Category, PublicPoolMap, PublicSlot},
};

#[derive(Debug, Clone)]
pub struct PoolEntry {
    pub map_id: Uuid,
    pub beatmap_id: i64,
    pub beatmapset_id: i64,
    // The mods the room plays it with, as acronyms (an FM or TB map's: none).
    pub mods: String,
    pub mode: String,
    // Its category is freemod or the tiebreaker, or the map was added as FM or TB.
    pub freemod: bool,
    pub tiebreaker: bool,
    // Its slot as players and the pages name it: its category's short name and its
    // place, "HD1", or just "TB" for a category of one slot (matchPool.ts's poolEntries).
    pub label: String,
    // Its category's short name: a team can't pick from the same one twice in a row
    // unless it's `repeatable` (NoMod; see autoref/draft.rs).
    pub category: String,
    pub repeatable: bool,
}

// The pool in category and slot order.
pub fn pool_entries(
    categories: &[Category],
    slots: &[PublicSlot],
    maps: &[PublicPoolMap],
) -> Vec<PoolEntry> {
    let mut entries = Vec::new();
    for category in categories {
        let mut own: Vec<&PublicSlot> = slots
            .iter()
            .filter(|s| s.category_id == category.id)
            .collect();
        own.sort_by_key(|s| s.position);
        let several = own.len() > 1;
        for (i, slot) in own.into_iter().enumerate() {
            let Some(map) = maps.iter().find(|m| m.slot_id == Some(slot.id)) else {
                continue;
            };
            let mark = map.mods.trim().to_uppercase();
            entries.push(PoolEntry {
                map_id: map.id,
                beatmap_id: map.beatmap_id,
                beatmapset_id: map.beatmapset_id,
                mods: category::playable_mods(&map.mods).to_string(),
                mode: map.mode.clone(),
                freemod: category.kind.freemod() || category::is_kind_mark(&mark),
                tiebreaker: category.kind.tiebreaker() || mark == category::TIEBREAKER_MAP,
                label: if several {
                    format!("{}{}", category.abbreviation, i + 1)
                } else {
                    category.abbreviation.clone()
                },
                category: category.abbreviation.clone(),
                repeatable: category.kind.repeatable(),
            });
        }
    }
    entries
}

// Mods compared without NoFail, Nightcore as the DoubleTime it is, in a fixed order.
fn mod_key(acronyms: &str) -> String {
    let mut mods: Vec<String> = split_mods(acronyms)
        .into_iter()
        .filter(|m| m != "NF")
        .map(|m| if m == "NC" { "DT".into() } else { m })
        .collect();
    mods.sort();
    mods.concat()
}

// The pool entry a finished map was: the one with its beatmap. When the pool has that
// beatmap more than once, the one whose mods it was played with, then the one a team
// picked, then the first.
pub fn entry_of<'a>(
    entries: &'a [PoolEntry],
    beatmap_id: Option<i64>,
    room_mods: &str,
    freemod: bool,
    picked: &HashSet<Uuid>,
) -> Option<&'a PoolEntry> {
    let candidates: Vec<&PoolEntry> = entries
        .iter()
        .filter(|e| Some(e.beatmap_id) == beatmap_id)
        .collect();
    if candidates.len() <= 1 {
        return candidates.first().copied();
    }
    let played = mod_key(room_mods);
    let by_mods: Vec<&PoolEntry> = candidates
        .iter()
        .copied()
        .filter(|e| {
            if freemod {
                e.freemod
            } else {
                !e.freemod && mod_key(&e.mods) == played
            }
        })
        .collect();
    let pool = if by_mods.is_empty() {
        candidates
    } else {
        by_mods
    };
    pool.iter()
        .copied()
        .find(|e| picked.contains(&e.map_id))
        .or(pool.first().copied())
}
