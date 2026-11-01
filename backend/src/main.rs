mod auth;
mod bancho;
mod config;
mod db;
mod discord;
mod discord_api;
mod draft;
mod error;
mod handlers;
mod mods;
mod osu_api;
mod pickem;
mod potentials;
mod rank_refresh;
mod role;
mod room_scores;
mod stats;
mod team_rank;

use axum::Router;
use clap::Parser;
use config::Config;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use sqlx::PgPool;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
};

// csrf_tokens only guards the few seconds of an OAuth handshake; postgres holds the real user state.
#[derive(Clone)]
pub struct AppState {
    pub osu_client: BasicClient,
    pub discord_client: BasicClient,
    pub http_client: reqwest::Client,
    pub db: PgPool,
    pub csrf_tokens: Arc<Mutex<HashMap<String, ()>>>,
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
    pub irc_users: handlers::irc::IrcUsers,
    // Every open page's live socket: score pushes and referee-page presence.
    pub live: handlers::live::Live,
    // The tournament server's bot, when it's configured.
    pub discord: discord::Discord,
    // The statistics last counted, until the site next changes.
    pub stats_cache: handlers::stats::StatsCache,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Arc::new(Config::parse());

    let osu_client = BasicClient::new(
        ClientId::new(config.osu_client_id.clone()),
        Some(ClientSecret::new(config.osu_client_secret.clone())),
        AuthUrl::new("https://osu.ppy.sh/oauth/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://osu.ppy.sh/oauth/token".to_string()).unwrap()),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.osu_redirect_uri.clone()).expect("invalid OSU_REDIRECT_URI"),
    );

    let discord_client = BasicClient::new(
        ClientId::new(config.discord_client_id.clone()),
        Some(ClientSecret::new(config.discord_client_secret.clone())),
        AuthUrl::new("https://discord.com/api/oauth2/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://discord.com/api/oauth2/token".to_string()).unwrap()),
    )
    .set_redirect_uri(
        RedirectUrl::new(config.discord_redirect_uri.clone())
            .expect("invalid DISCORD_REDIRECT_URI"),
    );

    let db_pool = db::connect(&config.database_url).await;
    auth::spawn_session_cleanup(db_pool.clone());
    let (bancho, bancho_inbox) = bancho::Bancho::new();

    let state = AppState {
        osu_client,
        discord_client,
        http_client: reqwest::Client::new(),
        db: db_pool,
        csrf_tokens: Arc::new(Mutex::new(HashMap::new())),
        config: Arc::clone(&config),
        osu_app_token: Arc::new(Mutex::new(None)),
        rank_refresh_lock: Arc::new(tokio::sync::Mutex::new(())),
        bancho,
        irc_users: handlers::irc::IrcUsers::default(),
        live: handlers::live::Live::default(),
        discord: discord::Discord::new(&config.discord_bot),
        stats_cache: handlers::stats::StatsCache::default(),
    };
    handlers::stats::spawn_invalidation(&state);
    // Seeds loaded straight into the database, and bracket changes from before potential
    // matches existed, get theirs now.
    if let Err(err) = handlers::bracket::resync_potentials(&state).await {
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

    let listener = tokio::net::TcpListener::bind(&config.server_addr)
        .await
        .unwrap();
    tracing::info!("backend listening on http://{}", config.server_addr);
    axum::serve(listener, app).await.unwrap();
}
