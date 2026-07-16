//! Client for the Discord REST API (https://discord.com/api).
//!
//! Used only for optional account linking. One submodule per endpoint group;
//! functions take `&reqwest::Client` + a bearer token and surface failures as
//! `crate::error::AppError`.
pub mod user;

const API_BASE: &str = "https://discord.com/api";
