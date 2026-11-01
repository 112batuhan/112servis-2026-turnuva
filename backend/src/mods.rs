//! Deterministic mod stat transforms for the stats the osu! API's difficulty
//! attributes endpoint does NOT return (CS, AR, OD, BPM, length). Only the star rating
//! comes from the API. Modifiers are 2-letter acronyms concatenated in the map's
//! `mods` string, e.g. "HDDT".

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

// HR multiplies AR, OD (and HP) by 1.4 up to 10; EZ halves them.
fn scaled(value: f64, mods: &[String]) -> f64 {
    if has(mods, "HR") {
        (value * 1.4).min(10.0)
    } else if has(mods, "EZ") {
        value * 0.5
    } else {
        value
    }
}

// AR is a preempt time: 1800 ms at AR 0, 1200 ms at AR 5, 450 ms at AR 10. A rate
// change shortens or stretches that time, and the AR shown is the one it now matches,
// so DT can go past 10 (AR 10 → 11).
pub fn modded_ar(ar: f64, mods: &[String]) -> f64 {
    let ar = scaled(ar, mods);
    let preempt = if ar < 5.0 {
        1800.0 - 120.0 * ar
    } else {
        1200.0 - 150.0 * (ar - 5.0)
    } / rate(mods);
    let ar = if preempt > 1200.0 {
        (1800.0 - preempt) / 120.0
    } else {
        5.0 + (1200.0 - preempt) / 150.0
    };
    round2(ar)
}

// OD is the 300 hit window, 80 - 6 * OD ms, shortened or stretched by the rate the same
// way (OD 10 under DT is 11.11).
pub fn modded_od(od: f64, mods: &[String]) -> f64 {
    let window = (80.0 - 6.0 * scaled(od, mods)) / rate(mods);
    round2((80.0 - window) / 6.0)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str) -> Vec<String> {
        parse(s)
    }

    #[test]
    fn nomod_keeps_the_stats() {
        assert_eq!(modded_ar(9.3, &m("")), 9.3);
        assert_eq!(modded_od(8.5, &m("HD")), 8.5);
    }

    #[test]
    fn hard_rock_scales_and_caps() {
        assert_eq!(modded_ar(7.0, &m("HR")), 9.8);
        assert_eq!(modded_ar(9.2, &m("HR")), 10.0);
        assert_eq!(modded_od(8.0, &m("HDHR")), 10.0);
        assert_eq!(modded_cs(4.0, &m("HR")), 5.2);
    }

    #[test]
    fn double_time_goes_through_the_timings() {
        assert_eq!(modded_ar(8.5, &m("DT")), 10.0);
        assert_eq!(modded_ar(10.0, &m("NC")), 11.0);
        assert_eq!(modded_ar(9.0, &m("HDDT")), 10.33);
        assert_eq!(modded_ar(4.0, &m("DT")), 7.13);
        assert_eq!(modded_od(8.0, &m("DT")), 9.78);
        assert_eq!(modded_od(10.0, &m("DT")), 11.11);
    }

    #[test]
    fn hard_rock_applies_before_the_rate() {
        assert_eq!(modded_ar(9.0, &m("DTHR")), 11.0);
        assert_eq!(modded_od(8.0, &m("HRDT")), 11.11);
    }

    #[test]
    fn half_time_and_easy_slow_it_down() {
        assert_eq!(modded_ar(9.0, &m("HT")), 7.67);
        assert_eq!(modded_od(8.0, &m("HT")), 6.22);
        assert_eq!(modded_ar(8.0, &m("EZ")), 4.0);
    }
}
