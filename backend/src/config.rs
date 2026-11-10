use clap::Parser;

/// Backend configuration. Populated from environment variables (see .env.example),
/// which can also be overridden with matching CLI flags, e.g. `--server-addr`.
#[derive(Parser, Debug, Clone)]
#[command(name = "backend", author, version, about)]
pub struct Config {
    /// Postgres connection string; migrations run automatically on startup.
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// The most connections the backend keeps to Postgres. The Discord job worker's
    /// listener holds one of them for good.
    #[arg(long, env = "DATABASE_MAX_CONNECTIONS", default_value_t = 20)]
    pub database_max_connections: u32,

    /// Encrypts the bots' IRC passwords in the database: 32 bytes as 64 hex digits
    /// (`openssl rand -hex 32`). Needed once any bot is saved; changing it loses the
    /// saved passwords (hosts enter them again).
    #[arg(long, env = "BOT_SECRET_KEY")]
    pub bot_secret_key: Option<String>,

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

    #[command(flatten)]
    pub arg: ArgConfig,

    /// Bancho's IRC server, which the bots and the referee page's chat relay connect to.
    /// cho.ppy.sh:6667 is an alternative if irc.ppy.sh doesn't work.
    #[arg(long, env = "BANCHO_IRC_ADDR", default_value = "irc.ppy.sh:6667")]
    pub bancho_irc_addr: String,

    /// Enables /api/dev/* (sign in as any account, seed dummy data). It works over https
    /// too, so testers can use it on the live server before launch (the server only logs
    /// a warning), but anyone can then sign in as anyone: turn it off before launch.
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
    pub discord_guild_id: Option<String>,
}

impl DiscordBotConfig {
    pub fn bot(&self) -> Option<(&str, u64)> {
        // Empty counts as unset (docker-compose passes unset variables as empty).
        let token = set(&self.discord_bot_token)?;
        let guild_id = match set(&self.discord_guild_id)?.trim().parse() {
            Ok(id) => id,
            Err(_) => {
                tracing::error!("DISCORD_GUILD_ID isn't a number; the Discord bot is off");
                return None;
            }
        };
        Some((token, guild_id))
    }
}

/// The hidden puzzle behind the snowflake in the landing page's banner (handlers/arg.rs): three secret messages,
/// each answered by its reply, entered in any order; the first to enter the third wins.
/// Off unless every answer and reply is set. Answers are matched ignoring case and extra
/// spaces; a reply's `{place}` becomes the player's place among that message's finders.
/// Changing them needs a restart.
#[derive(clap::Args, Debug, Clone)]
pub struct ArgConfig {
    #[arg(long, env = "ARG_ANSWER_1")]
    pub arg_answer_1: Option<String>,
    #[arg(long, env = "ARG_REPLY_1")]
    pub arg_reply_1: Option<String>,
    #[arg(long, env = "ARG_ANSWER_2")]
    pub arg_answer_2: Option<String>,
    #[arg(long, env = "ARG_REPLY_2")]
    pub arg_reply_2: Option<String>,
    #[arg(long, env = "ARG_ANSWER_3")]
    pub arg_answer_3: Option<String>,
    #[arg(long, env = "ARG_REPLY_3")]
    pub arg_reply_3: Option<String>,

    /// Opens the results page at /api/kupa/results/<token>. Unset, there's no results page.
    #[arg(long, env = "ARG_RESULTS_TOKEN")]
    pub arg_results_token: Option<String>,
}

/// One message of the puzzle: what to enter and what it answers.
pub struct ArgMessage<'a> {
    pub answer: &'a str,
    pub reply: &'a str,
}

// An empty variable (as docker-compose passes an unset one) counts as unset.
fn set(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.trim().is_empty())
}

impl ArgConfig {
    /// The three messages, the winning one last, or None unless all of them are set.
    pub fn messages(&self) -> Option<[ArgMessage<'_>; 3]> {
        let message = |answer, reply| {
            Some(ArgMessage {
                answer: set(answer)?,
                reply: set(reply)?,
            })
        };
        Some([
            message(&self.arg_answer_1, &self.arg_reply_1)?,
            message(&self.arg_answer_2, &self.arg_reply_2)?,
            message(&self.arg_answer_3, &self.arg_reply_3)?,
        ])
    }

    pub fn results_token(&self) -> Option<&str> {
        set(&self.arg_results_token)
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
