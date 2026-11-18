//! A match's finished maps and score, counted from its rooms: each finished pool map
//! goes to the team with the higher total, unless the referee left it out ("Sayma"),
//! plus the maps they awarded by hand. The server counts it, saves it and shows it to
//! every page, so no page has to.

mod count;
mod pool;
mod results;
mod standings;
#[cfg(test)]
mod tests;

pub use count::{lobby_key, lobby_rows, row_of, score_key, tally, TallyInput};
pub use pool::{entry_of, pool_entries, PoolEntry};
pub use results::{
    osu_results, room_results, scorer, team_totals, Finished, RosterPlayer, TeamTotal,
};
pub use standings::{score_message, standing_of, turn_of, Standing};
