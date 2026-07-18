//! Deterministic mod stat transforms for the stats the osu! API's difficulty
//! attributes endpoint does NOT return (CS, HP, BPM, length). Star rating, AR, and
//! OD come from the API instead. Modifiers are 2-letter acronyms concatenated in the
//! category's `modifier` string, e.g. "HDDT".

fn has(mods: &[String], code: &str) -> bool {
    mods.iter().any(|m| m == code)
}

// Splits a modifier string like "HDDT" into ["HD", "DT"].
pub fn parse(modifier: &str) -> Vec<String> {
    let upper = modifier.to_uppercase();
    upper
        .as_bytes()
        .chunks(2)
        .filter_map(|c| std::str::from_utf8(c).ok())
        .filter(|s| s.len() == 2)
        .map(str::to_string)
        .collect()
}

// Playback rate multiplier: DT/NC speed up 1.5x, HT slows to 0.75x.
pub fn rate(mods: &[String]) -> f64 {
    if has(mods, "DT") || has(mods, "NC") {
        1.5
    } else if has(mods, "HT") {
        0.75
    } else {
        1.0
    }
}

pub fn modded_cs(cs: f64, mods: &[String]) -> f64 {
    if has(mods, "HR") {
        (cs * 1.3).min(10.0)
    } else if has(mods, "EZ") {
        cs * 0.5
    } else {
        cs
    }
}

pub fn modded_hp(hp: f64, mods: &[String]) -> f64 {
    if has(mods, "HR") {
        (hp * 1.4).min(10.0)
    } else if has(mods, "EZ") {
        hp * 0.5
    } else {
        hp
    }
}
