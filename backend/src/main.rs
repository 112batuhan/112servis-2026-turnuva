mod auth;
mod bancho;
mod config;
mod db;
mod discord;
mod discord_api;
mod draft;
mod error;
mod handlers;
mod live;
mod mods;
mod osu_api;
mod pickem;
mod potentials;
mod rank_refresh;
mod role;
mod room_scores;
mod secret;
mod seeding;
mod services;
mod stage_track;
mod stats;
mod stats_cache;
mod supervise;
mod team_rank;
mod views;

use axum::Router;
use clap::Parser;
use config::Config;
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, EndpointNotSet, EndpointSet, RedirectUrl,
    TokenUrl,
};
use sqlx::PgPool;
use std::{
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tower_http::{
    compression::{
        predicate::{DefaultPredicate, Predicate},
        CompressionLayer,
    },
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer},
};

// An OAuth client with its authorization and token endpoints set.
pub type OAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

// osu! and Discord requests give up after these, so a hung one can't hold anything
// (a rank refresh holds its lock while it calls osu!).
const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// oauth_states only guards the few minutes of an OAuth handshake; postgres holds the
// real user state.
#[derive(Clone)]
pub struct AppState {
    pub osu_client: OAuthClient,
    pub discord_client: OAuthClient,
    // For osu! and Discord API calls.
    pub http_client: reqwest::Client,
    // For OAuth token exchanges (no redirects, as oauth2 asks), and the profile
    // request right after each, which reuses the exchange's connection.
    pub oauth_http: reqwest::Client,
    pub db: PgPool,
    pub oauth_states: auth::OAuthStates,
    // Encrypts the bots' IRC passwords in the database; None without BOT_SECRET_KEY.
    pub bot_secret: Option<Arc<secret::BotSecret>>,
    // Startup configuration (env-derived). Behind an Arc so cloning AppState — which
    // axum does per request — is a cheap refcount bump rather than copying strings.
    pub config: Arc<Config>,
    // osu! app access token (client-credentials grant) for public API calls.
    pub osu_app_token: Arc<Mutex<Option<osu_api::token::OsuAppToken>>>,
    // Held while every team member's rank is being refreshed.
    pub rank_refresh_lock: Arc<tokio::sync::Mutex<()>>,
    // The site's bots on Bancho, which run every multiplayer room.
    pub bancho: bancho::Bancho,
    // Each referee's own Bancho IRC connection, shared by their tabs, and failed logins.
    pub irc_users: bancho::relay::IrcUsers,
    // Every open page's live socket: score pushes and referee-page presence.
    pub live: live::Live,
    // The tournament server's bot, when it's configured.
    pub discord: discord::Discord,
    // The statistics last counted, until the site next changes.
    pub stats_cache: stats_cache::StatsCache,
    // The public bracket last read, likewise.
    pub bracket_cache: stats_cache::StatsCache,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Arc::new(Config::parse());

    let osu_client = BasicClient::new(ClientId::new(config.osu_client_id.clone()))
        .set_client_secret(ClientSecret::new(config.osu_client_secret.clone()))
        .set_auth_uri(AuthUrl::new("https://osu.ppy.sh/oauth/authorize".to_string()).unwrap())
        .set_token_uri(TokenUrl::new("https://osu.ppy.sh/oauth/token".to_string()).unwrap())
        .set_redirect_uri(
            RedirectUrl::new(config.osu_redirect_uri.clone()).expect("invalid OSU_REDIRECT_URI"),
        );

    let discord_client = BasicClient::new(ClientId::new(config.discord_client_id.clone()))
        .set_client_secret(ClientSecret::new(config.discord_client_secret.clone()))
        .set_auth_uri(AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).unwrap())
        .set_token_uri(TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap())
        .set_redirect_uri(
            RedirectUrl::new(config.discord_redirect_uri.clone())
                .expect("invalid DISCORD_REDIRECT_URI"),
        );

    let db_pool = db::connect(&config.database_url, config.database_max_connections).await;
    let bot_secret = bot_secret(&config, &db_pool).await;
    auth::spawn_session_cleanup(db_pool.clone());
    let (bancho, bancho_inbox) = bancho::Bancho::new();

    let state = AppState {
        osu_client,
        discord_client,
        http_client: reqwest::Client::builder()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .timeout(HTTP_REQUEST_TIMEOUT)
            .build()
            .expect("could not build the http client"),
        oauth_http: reqwest::Client::builder()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .timeout(HTTP_REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("could not build the oauth http client"),
        db: db_pool,
        oauth_states: auth::OAuthStates::default(),
        bot_secret,
        config: Arc::clone(&config),
        osu_app_token: Arc::new(Mutex::new(None)),
        rank_refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
        bancho,
        irc_users: bancho::relay::IrcUsers::default(),
        live: live::Live::default(),
        discord: discord::Discord::new(&config.discord_bot),
        stats_cache: stats_cache::StatsCache::stats(),
        bracket_cache: stats_cache::StatsCache::bracket(),
    };
    stats_cache::spawn_invalidation(&state);
    // Seeds loaded straight into the database, and bracket changes from before potential
    // matches existed, get theirs now.
    if let Err(err) = services::bracket::resync_potentials(&state).await {
        tracing::error!(error = ?err, "could not work out the potential matches");
    }
    rank_refresh::spawn_daily_refresh(state.clone());
    discord::spawn(&state).await;
    bancho::spawn(&state, bancho_inbox);

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .frontend_url
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_credentials(true)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let mut api = Router::new()
        .merge(handlers::user::routes())
        .merge(handlers::admin::routes())
        .merge(handlers::arg::routes())
        .merge(handlers::mappool::routes())
        .merge(handlers::matches::routes())
        .merge(handlers::bracket::routes())
        .merge(handlers::referee::routes())
        .merge(handlers::reschedule::routes())
        .merge(handlers::rules::routes())
        .merge(handlers::seeding::routes())
        .merge(handlers::stages::routes())
        .merge(handlers::stats::routes())
        .merge(handlers::notifications::routes())
        .merge(handlers::pickem::routes())
        .merge(handlers::irc::routes())
        .merge(handlers::bancho::routes())
        .merge(handlers::live::routes())
        .merge(handlers::discord::routes())
        .merge(handlers::team::routes());
    if config.dev_login {
        tracing::warn!("DEV_LOGIN is on: anyone can sign in as any account via /api/dev");
        api = api.merge(handlers::dev::routes());
    }

    let mut app = Router::new()
        .nest("/auth", handlers::auth::routes())
        .nest("/api", api)
        .layer(cors)
        .with_state(state);

    // Serves the built frontend (see STATIC_DIR) when present, falling back to
    // index.html for client-side routes so a single container can host both.
    if Path::new(&config.static_dir).join("index.html").exists() {
        let index_file = ServeFile::new(Path::new(&config.static_dir).join("index.html"));
        app = app.fallback_service(ServeDir::new(&config.static_dir).not_found_service(index_file));
    }

    // Responses are compressed here unless a reverse proxy in front does it; a WebSocket
    // upgrade never is.
    let compress =
        DefaultPredicate::new().and(|status: axum::http::StatusCode, _: _, _: &_, _: &_| {
            status != axum::http::StatusCode::SWITCHING_PROTOCOLS
        });
    let app = app
        .layer(CompressionLayer::new().compress_when(compress))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::new().level(tracing::Level::INFO))
                .on_response(DefaultOnResponse::new().level(tracing::Level::INFO)),
        );

    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .unwrap();
    tracing::info!("backend listening on http://{}", config.server_addr);
    let server = axum::serve(listener, app).with_graceful_shutdown(async {
        shutdown_signal().await;
        tracing::info!("shutdown requested");
    });
    tokio::select! {
        result = server => result.unwrap(),
        // Open WebSockets would keep it waiting, so requests get a few seconds to finish
        // (inside Docker's 10 s stop timeout) and the rest are cut off.
        () = async {
            shutdown_signal().await;
            tokio::time::sleep(SHUTDOWN_GRACE).await;
        } => tracing::warn!("shutting down with connections still open"),
    }
    tracing::info!("backend stopped");
}

// Reads BOT_SECRET_KEY and encrypts the bot passwords still saved in plaintext. A key
// that isn't 64 hex digits stops the server rather than leaving the bots offline.
async fn bot_secret(config: &Config, db: &PgPool) -> Option<Arc<secret::BotSecret>> {
    let key = config
        .bot_secret_key
        .as_deref()
        .filter(|key| !key.trim().is_empty());
    let Some(key) = key else {
        match db::bancho::count_plaintext(db).await {
            Ok(0) => {}
            Ok(n) => tracing::warn!(
                "{n} bot passwords are saved in plaintext; set BOT_SECRET_KEY to encrypt them"
            ),
            Err(err) => tracing::error!(error = ?err, "could not read the bot accounts"),
        }
        return None;
    };
    let secret = secret::BotSecret::from_hex(key)
        .expect("BOT_SECRET_KEY must be 64 hex digits (openssl rand -hex 32)");
    match db::bancho::encrypt_plaintext(db, &secret).await {
        Ok(0) => {}
        Ok(n) => tracing::info!("encrypted {n} bot passwords"),
        Err(err) => tracing::error!(error = ?err, "could not encrypt the bot passwords"),
    }
    Some(Arc::new(secret))
}

// How long requests in flight get to finish once the backend is told to stop.
const SHUTDOWN_GRACE: Duration = Duration::from_secs(8);

// Resolves on ctrl-c or SIGTERM (what `docker stop` sends).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("could not listen for ctrl-c");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("could not listen for SIGTERM")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = ctrl_c => {}
        _ = terminate => {}
    }
}
