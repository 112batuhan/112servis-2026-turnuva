// Team eligibility formula, from the "2026 Oyuncu kayıt formül" and
// "Oyuncu rank hesap ters" Desmos sheets. Each player's rank maps to a pp-like
// value; a team is allowed while the sum of both values stays at or below LIMIT.

// Shifts every rank before the curve is applied (the sheets' `k`).
pub const K: f64 = 175.0;
// Highest allowed team value (the sheets' `p`).
pub const LIMIT: f64 = 19000.0;

pub fn player_value(rank: i64) -> f64 {
    let r = rank as f64 + K;
    26911.0 - 1890.0 * r.ln() - 1890.0 * K / r
}

pub fn team_value(a: i64, b: i64) -> f64 {
    player_value(a) + player_value(b)
}

pub fn is_allowed(a: i64, b: i64) -> bool {
    team_value(a, b) <= LIMIT
}

// The best (lowest) rank a partner of `rank` may have. player_value falls as the
// rank grows, so a binary search over ranks gives the same answer as the Newton
// solver in the inverse sheet.
pub fn min_partner_rank(rank: i64) -> i64 {
    let (mut lo, mut hi) = (1_i64, 100_000_000_i64);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if is_allowed(rank, mid) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn min_partner_rank_is_the_boundary() {
        let rank = 6400;
        let min = min_partner_rank(rank);
        assert!(is_allowed(rank, min));
        assert!(!is_allowed(rank, min - 1));
    }
}
