#![allow(clippy::multiple_crate_versions)]
//! Homelab Dashboard Server
//!
//! Axum server that renders the dashboard with Maud and drives htmx over a
//! shared Server-Sent Events stream. Each card is delivered two ways: pushed
//! over `/events` (swapped by event name) and served from `/card/{id}` for
//! manual refresh. Search is proxied to `SearXNG` via `/search`.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::time::Duration;

use axum::Router;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use color_eyre::eyre::Result;
use futures::Stream;
use maud::Markup;
use tower_http::compression::CompressionLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use dashboard_app::DashboardConfig;
use dashboard_app::fetch::search;
use dashboard_app::views;

/// How often the shared SSE stream re-renders every card. Per-service `moka`
/// caches dedupe upstream calls, so this can stay snappy.
const SSE_INTERVAL: Duration = Duration::from_secs(15);

/// Card ids in display order. Each doubles as the SSE event name and the
/// `/card/{id}` route segment.
const CARD_IDS: [&str; 6] = [
    "weather",
    "video",
    "proxmox",
    "jellyfin",
    "homeassistant",
    "health",
];

/// Shared handler state: the display config the fetchers need per render.
#[derive(Clone)]
struct AppState {
    config: DashboardConfig,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "dashboard_server=info,dashboard_app=info,tower_http=info".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = load_config()?;

    tracing::info!(
        "Starting dashboard server on {}:{}",
        config.listen_address,
        config.port
    );

    let addr = SocketAddr::new(config.listen_address.parse()?, config.port);

    // Service URLs and secrets are consumed by the fetchers from the
    // environment; push the loaded config through before any handler runs.
    apply_config_to_env(&config);

    let state = AppState {
        config: DashboardConfig {
            latitude: config.weather.latitude,
            longitude: config.weather.longitude,
            location_name: config.weather.location.clone(),
            searxng_url: config.searxng.url.clone(),
            ha_entity_ids: config.homeassistant.entity_ids.clone(),
        },
    };

    // Static assets (compiled dashboard.css, etc.). Defaults to ./static for
    // local dev; the Nix package points this at its share directory.
    let static_dir = std::env::var("DASHBOARD_STATIC_DIR").unwrap_or_else(|_| "static".to_string());

    let app = Router::new()
        .route("/", get(index))
        .route("/events", get(events))
        .route("/card/{id}", get(card))
        .route("/search", get(search_handler))
        .fallback_service(ServeDir::new(static_dir))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Dashboard available at http://{}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

/// The full dashboard page (skeleton cards until the first SSE frame).
async fn index(State(state): State<AppState>) -> Markup {
    views::page(&state.config)
}

/// A single card partial for manual refresh; 404 for unknown ids.
#[allow(clippy::option_if_let_else)]
async fn card(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    match render_card(&state.config, &id).await {
        Some(markup) => markup.into_response(),
        None => (StatusCode::NOT_FOUND, "unknown card").into_response(),
    }
}

/// Shared SSE stream: renders every card each [`SSE_INTERVAL`], but only pushes
/// a frame when a card's markup actually changed. Unchanged cards aren't
/// re-sent, so htmx doesn't morph them and the browser keeps their assets.
async fn events(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let cfg = state.config;

    let stream = async_stream::stream! {
        // Last markup emitted per card id; a card is re-pushed only on change.
        let mut last: HashMap<&str, String> = HashMap::new();
        loop {
            for id in CARD_IDS {
                if let Some(markup) = render_card(&cfg, id).await {
                    let html = markup.into_string();
                    if last.get(id).is_some_and(|prev| *prev == html) {
                        continue;
                    }
                    yield Ok(Event::default().event(id).data(&html));
                    last.insert(id, html);
                }
            }
            tokio::time::sleep(SSE_INTERVAL).await;
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// Proxy a search query to `SearXNG` and render the results dropdown.
async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Markup {
    let query = params.q.unwrap_or_default();

    if query.trim().is_empty() {
        return views::search_results(&query, &[]);
    }

    match search(state.config.searxng_url.as_deref(), &query).await {
        Ok(response) => views::search_results(&query, &response.results),
        Err(e) => views::search_error(&e.to_string()),
    }
}

/// Query string for `/search`.
#[derive(Debug, serde::Deserialize)]
struct SearchParams {
    q: Option<String>,
}

/// Render the inner partial for a card by id (`None` if the id is unknown).
async fn render_card(cfg: &DashboardConfig, id: &str) -> Option<Markup> {
    let markup = match id {
        "weather" => views::weather_card(cfg).await,
        "video" => views::video_card().await,
        "proxmox" => views::proxmox_card().await,
        "jellyfin" => views::jellyfin_card().await,
        "homeassistant" => views::homeassistant_card(cfg).await,
        "health" => views::health_card().await,
        _ => return None,
    };
    Some(markup)
}

// ============================================================================
// Configuration
// ============================================================================

#[derive(Debug, serde::Deserialize)]
struct ServerConfig {
    #[serde(default = "default_listen_address")]
    listen_address: String,
    #[serde(default = "default_port")]
    port: u16,

    #[serde(default)]
    searxng: SearxngConfig,
    #[serde(default)]
    weather: WeatherConfig,
    #[serde(default)]
    proxmox: ProxmoxConfig,
    #[serde(default)]
    jellyfin: JellyfinConfig,
    #[serde(default)]
    homeassistant: HomeAssistantConfig,
    #[serde(default)]
    hofvarpnir: HofvarpnirConfig,
    #[serde(default)]
    health_checks: Vec<HealthCheckConfig>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct SearxngConfig {
    url: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct WeatherConfig {
    #[serde(default = "default_latitude")]
    latitude: f64,
    #[serde(default = "default_longitude")]
    longitude: f64,
    #[serde(default = "default_location_name")]
    location: String,
}

impl Default for WeatherConfig {
    fn default() -> Self {
        Self {
            latitude: default_latitude(),
            longitude: default_longitude(),
            location: default_location_name(),
        }
    }
}

#[derive(Debug, Default, serde::Deserialize)]
struct ProxmoxConfig {
    url: Option<String>,
    token_id: Option<String>,
    token_secret: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct JellyfinConfig {
    url: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct HomeAssistantConfig {
    url: Option<String>,
    token: Option<String>,
    #[serde(default)]
    entity_ids: Vec<String>,
}

#[derive(Debug, Default, serde::Deserialize)]
struct HofvarpnirConfig {
    url: Option<String>,
    api_key: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct HealthCheckConfig {
    name: String,
    url: String,
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
    expected_status: Option<u16>,
}

fn default_listen_address() -> String {
    "0.0.0.0".to_string()
}

const fn default_port() -> u16 {
    8080
}

const fn default_latitude() -> f64 {
    52.52
}

const fn default_longitude() -> f64 {
    13.41
}

fn default_location_name() -> String {
    "Berlin".to_string()
}

const fn default_timeout() -> u64 {
    5000
}

fn load_config() -> Result<ServerConfig> {
    use config::{Config, Environment, File};

    let config = Config::builder()
        .set_default("listen_address", "0.0.0.0")?
        .set_default("port", 8080)?
        .add_source(File::with_name("config").required(false))
        .add_source(File::with_name("/etc/homelab-dashboard/config").required(false))
        .add_source(
            Environment::with_prefix("DASHBOARD")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    Ok(config.try_deserialize()?)
}

/// Push service URLs and secrets into the environment for the fetchers to read.
fn apply_config_to_env(config: &ServerConfig) {
    /// Set an env var (the `config` crate already validated these values).
    fn set(key: &str, value: &str) {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    if let Some(url) = &config.searxng.url {
        set("SEARXNG_URL", url);
    }

    set("WEATHER_LAT", &config.weather.latitude.to_string());
    set("WEATHER_LON", &config.weather.longitude.to_string());
    set("WEATHER_LOCATION", &config.weather.location);

    if let Some(url) = &config.proxmox.url {
        set("PROXMOX_URL", url);
    }
    if let Some(token_id) = &config.proxmox.token_id {
        set("PROXMOX_TOKEN_ID", token_id);
    }
    if let Some(token_secret) = &config.proxmox.token_secret {
        set("PROXMOX_TOKEN_SECRET", token_secret);
    }

    if let Some(url) = &config.jellyfin.url {
        set("JELLYFIN_URL", url);
    }
    if let Some(api_key) = &config.jellyfin.api_key {
        set("JELLYFIN_API_KEY", api_key);
    }

    if let Some(url) = &config.homeassistant.url {
        set("HOMEASSISTANT_URL", url);
    }
    if let Some(token) = &config.homeassistant.token {
        set("HOMEASSISTANT_TOKEN", token);
    }

    if let Some(url) = &config.hofvarpnir.url {
        set("HOFVARPNIR_URL", url);
    }
    if let Some(api_key) = &config.hofvarpnir.api_key {
        set("HOFVARPNIR_API_KEY", api_key);
    }

    if !config.health_checks.is_empty()
        && let Ok(json) = serde_json::to_string(&config.health_checks)
    {
        set("HEALTH_CHECKS", json.as_str());
    }
}
