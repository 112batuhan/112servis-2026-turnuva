use clap::Parser;

/// Backend configuration. Populated from environment variables (see .env.example),
/// which can also be overridden with matching CLI flags, e.g. `--server-addr`.
#[derive(Parser, Debug, Clone)]
#[command(name = "backend", author, version, about)]
pub struct Config {
    /// Postgres connection string; migrations run automatically on startup.
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

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

    #[command(flatten)]
    pub discord_bot: DiscordBotConfig,

    /// Bancho's IRC server, which the bots and the referee page's chat relay connect to.
    /// cho.ppy.sh:6667 is an alternative if irc.ppy.sh doesn't work.
    #[arg(long, env = "BANCHO_IRC_ADDR", default_value = "irc.ppy.sh:6667")]
    pub bancho_irc_addr: String,

    /// Enables /api/dev/* (sign in as any account, seed dummy data). Local testing only;
    /// the server refuses to start with it over https.
    #[arg(long, env = "DEV_LOGIN", default_value_t = false, action = clap::ArgAction::Set)]
    pub dev_login: bool,
}

/// The tournament server's Discord bot. It runs only when both are set; everything it
/// points at in the server (roles, channels, categories) is picked on /admin/discord.
#[derive(clap::Args, Debug, Clone)]
pub struct DiscordBotConfig {
    /// The bot's token, from the Discord application's Bot page. Needs the Server
    /// Members intent.
    #[arg(long, env = "DISCORD_BOT_TOKEN")]
    pub discord_bot_token: Option<String>,

    /// The tournament server's id.
    #[arg(long, env = "DISCORD_GUILD_ID")]
    pub discord_guild_id: Option<u64>,
}

impl DiscordBotConfig {
    pub fn bot(&self) -> Option<(&str, u64)> {
        Some((self.discord_bot_token.as_deref()?, self.discord_guild_id?))
    }
}

impl Config {
    /// Whether to mark the auth cookie `Secure`. Derived from FRONTEND_URL's scheme:
    /// true in production (https://...), false for local http:// dev, where a
    /// `Secure` cookie would be silently dropped by the browser. Kept as a derived
    /// method so `frontend_url` stays the single source of truth.
    pub fn secure_cookies(&self) -> bool {
        self.frontend_url.starts_with("https://")
    }
}
