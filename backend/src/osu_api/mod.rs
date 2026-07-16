//! Client for the osu! REST API (https://osu.ppy.sh/api/v2).
//!
//! One submodule per endpoint group; functions take `&reqwest::Client` + a bearer
//! token and surface failures as `crate::error::AppError`.
pub mod user;

const API_BASE: &str = "https://osu.ppy.sh/api/v2";
