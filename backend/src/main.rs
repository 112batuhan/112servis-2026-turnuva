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
    routing::{get, post},
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
    pub jwt_secret: Arc<str>,
    pub frontend_url: String,
}

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt::init();

    let config = Config::parse();

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
        jwt_secret: Arc::from(config.jwt_secret.as_str()),
        frontend_url: config.frontend_url.clone(),
    };

    let cors = CorsLayer::new()
        .allow_origin(
            config
                .frontend_url
                .parse::<axum::http::HeaderValue>()
                .unwrap(),
        )
        .allow_credentials(true)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST])
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
        .route("/api/users", get(handlers::user::list_users))
        .route("/api/users/:id/role", post(handlers::user::set_role))
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
