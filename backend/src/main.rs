mod auth;
mod config;
mod db;
mod discord_api;
mod error;
mod handlers;
mod jwt;
mod osu_api;
mod role;

use axum::{
    routing::{delete, get, patch, post},
    Router,
};
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

    let state = AppState {
        osu_client,
        discord_client,
        http_client: reqwest::Client::new(),
        db: db_pool,
        csrf_tokens: Arc::new(Mutex::new(HashMap::new())),
        config: Arc::clone(&config),
        osu_app_token: Arc::new(Mutex::new(None)),
    };

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
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([axum::http::header::CONTENT_TYPE]);

    let mut app = Router::new()
        .route("/auth/osu", get(handlers::auth::osu_login))
        .route("/auth/osu/callback", get(handlers::auth::osu_callback))
        .route("/auth/discord/link", get(handlers::auth::discord_link))
        .route(
            "/auth/discord/callback",
            get(handlers::auth::discord_callback),
        )
        .route("/auth/logout", post(handlers::user::logout))
        .route("/api/me", get(handlers::user::me))
        .route("/api/users", get(handlers::admin::list_users))
        .route("/api/users/:id/role", post(handlers::admin::set_role))
        // Map pools (map_pooler+).
        .route(
            "/api/stages",
            get(handlers::mappool::list_stages).post(handlers::mappool::create_stage),
        )
        .route(
            "/api/stages/:id",
            get(handlers::mappool::get_stage)
                .delete(handlers::mappool::delete_stage)
                .patch(handlers::mappool::set_published),
        )
        .route(
            "/api/stages/:id/categories",
            post(handlers::mappool::create_category),
        )
        .route(
            "/api/categories/:id",
            delete(handlers::mappool::delete_category),
        )
        // Global generic pool (shared library).
        .route("/api/pool", post(handlers::mappool::add_to_generic))
        .route(
            "/api/pool/:beatmap_id",
            delete(handlers::mappool::remove_from_generic),
        )
        // Per-stage categorised placements.
        .route(
            "/api/stages/:id/entries",
            post(handlers::mappool::add_entry),
        )
        .route(
            "/api/entries/:id",
            patch(handlers::mappool::move_entry).delete(handlers::mappool::delete_entry),
        )
        // Public, unauthenticated: published stages only.
        .route(
            "/api/public/stages",
            get(handlers::mappool::list_public_stages),
        )
        .route(
            "/api/public/stages/:id",
            get(handlers::mappool::get_public_stage),
        )
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
