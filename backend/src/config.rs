use clap::Parser;

/// Backend configuration. Populated from environment variables (see .env.example),
/// which can also be overridden with matching CLI flags, e.g. `--server-addr`.
#[derive(Parser, Debug, Clone)]
#[command(name = "backend", author, version, about)]
pub struct Config {
    /// Postgres connection string; migrations run automatically on startup.
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// Secret used to sign login JWTs, e.g. `openssl rand -hex 32`.
    #[arg(long, env = "JWT_SECRET")]
    pub jwt_secret: String,

    /// Where the frontend lives (used for CORS + post-login redirect).
    #[arg(long, env = "FRONTEND_URL", default_value = "http://localhost:5173")]
    pub frontend_url: String,

    #[arg(long, env = "SERVER_ADDR", default_value = "127.0.0.1:8080")]
    pub server_addr: String,

    /// Directory containing a built frontend to serve as a SPA alongside the API.
    #[arg(long, env = "STATIC_DIR", default_value = "./static")]
    pub static_dir: String,

    /// osu! OAuth app: https://osu.ppy.sh/home/account/edit#oauth
    #[arg(long, env = "OSU_CLIENT_ID")]
    pub osu_client_id: String,

    #[arg(long, env = "OSU_CLIENT_SECRET")]
    pub osu_client_secret: String,

    #[arg(long, env = "OSU_REDIRECT_URI")]
    pub osu_redirect_uri: String,

    /// Discord OAuth app, used only for optional account linking: https://discord.com/developers/applications
    #[arg(long, env = "DISCORD_CLIENT_ID")]
    pub discord_client_id: String,

    #[arg(long, env = "DISCORD_CLIENT_SECRET")]
    pub discord_client_secret: String,

    #[arg(long, env = "DISCORD_REDIRECT_URI")]
    pub discord_redirect_uri: String,
}
