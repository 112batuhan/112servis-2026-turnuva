//! Mods as BanchoBot names them and as the site writes them.

// BanchoBot's mod names, as osu! abbreviates them.
fn mod_acronym(name: &str) -> String {
    match name {
        "nofail" => "NF",
        "easy" => "EZ",
        "touchdevice" => "TD",
        "hidden" => "HD",
        "hardrock" => "HR",
        "suddendeath" => "SD",
        "doubletime" => "DT",
        "relax" => "RX",
        "halftime" => "HT",
        "nightcore" => "NC",
        "flashlight" => "FL",
        "spunout" => "SO",
        "relax2" | "autopilot" => "AP",
        "perfect" => "PF",
        "fadein" => "FI",
        "mirror" => "MR",
        "scorev2" => "V2",
        other => return other.to_uppercase(),
    }
    .to_string()
}

// "Hidden, HardRock" (BanchoBot's names, freemod left out) as acronyms: "HDHR".
pub fn mod_acronyms(names: &str) -> String {
    names
        .split(',')
        .map(|n| n.split_whitespace().collect::<String>().to_lowercase())
        .filter(|n| !n.is_empty() && n != "freemod" && n != "none")
        .map(|n| mod_acronym(&n))
        .collect()
}

// "HDHR" -> ["HD", "HR"]; "NM" and "" are no mods.
pub fn split_mods(acronyms: &str) -> Vec<String> {
    let upper: Vec<char> = acronyms
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    upper
        .chunks(2)
        .filter(|c| c.len() == 2)
        .map(|c| c.iter().collect::<String>())
        .filter(|m| m != "NM")
        .collect()
}

// Whether the room's mods, as BanchoBot described them, include freemod.
pub fn is_freemod(mods: Option<&str>) -> bool {
    let lower = mods.unwrap_or_default().to_lowercase();
    lower.contains("freemod") || lower.contains("free mod")
}

// "Enabled Hidden, NoFail, disabled FreeMod" / "Disabled all mods, enabled FreeMod"
pub(super) fn describe_mods(text: &str) -> String {
    if text.to_lowercase().contains("enabled freemod") {
        return "Freemod".into();
    }
    match text.strip_prefix("Enabled ") {
        Some(rest) => match rest.find(", disabled ") {
            Some(i) => rest[..i].to_string(),
            None => rest.to_string(),
        },
        None => String::new(),
    }
}
