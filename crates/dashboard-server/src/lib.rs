#![allow(clippy::multiple_crate_versions)]
//! Homelab Dashboard server library.
//!
//! Holds everything the Axum binary wires together: the loaded [`ServerConfig`]
//! and its nested sections, the shared handler [`AppState`], the request
//! handlers, and [`build_router`] which assembles them into a [`Router`]. Each
//! card is delivered two ways: pushed over `/events` (swapped by event name)
//! and served from `/card/{id}` for manual refresh. The header search box
//! submits straight to the configured engine (e.g. `SearXNG`), so the dashboard
//! never proxies queries.
//!
//! The binary (`src/main.rs`) only does process setup — tracing, config
//! loading, socket binding — so this module stays importable from tests.

use std::collections::HashMap;
use std::convert::Infallible;
use std::time::Duration;

use axum::Router;
use axum::extract::{Path, State};
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

use dashboard_app::views;
use dashboard_app::{DashboardConfig, SearchEngine, SearchEngineKind};

/// How often the shared SSE stream re-renders every card. Per-service `moka`
/// caches dedupe upstream calls, so this can stay snappy.
pub const SSE_INTERVAL: Duration = Duration::from_secs(15);

/// Card ids in SSE push order. Each doubles as the SSE event name and the
/// `/card/{id}` route segment.
///
/// Deliberately a different order from [`dashboard_app::views::CARDS`], which
/// is display order; the two must stay equal as *sets*.
pub const CARD_IDS: [&str; 4] = ["weather", "video", "health", "links"];

/// Config files consulted by [`load_config`], in ascending precedence order.
/// Each is optional; extensions are resolved by the `config` crate.
const DEFAULT_CONFIG_PATHS: [&str; 2] = ["config", "/etc/homelab-dashboard/config"];

/// Shared handler state: the display config the fetchers need per render.
#[derive(Clone)]
pub struct AppState {
    /// Display config handed to every card view.
    pub config: DashboardConfig,
}

/// Assemble the dashboard router: routes, static-file fallback, and the
/// compression/tracing layers, with `state` attached.
///
/// `static_dir` is served for anything that doesn't match a route (the
/// compiled `dashboard.css` and friends).
pub fn build_router(state: AppState, static_dir: &str) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/events", get(events))
        .route("/card/{id}", get(card))
        .fallback_service(ServeDir::new(static_dir))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
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

/// Render the inner partial for a card by id (`None` if the id is unknown).
///
/// Ids are matched case-sensitively and must stay in sync with [`CARD_IDS`] and
/// [`dashboard_app::views::CARDS`].
pub async fn render_card(cfg: &DashboardConfig, id: &str) -> Option<Markup> {
    let markup = match id {
        "weather" => views::weather_card(cfg).await,
        "video" => views::video_card().await,
        "health" => views::health_card().await,
        "links" => views::quick_links_card(cfg),
        _ => return None,
    };
    Some(markup)
}

// ============================================================================
// Configuration
// ============================================================================

/// The whole `config.toml` (plus `DASHBOARD__*` env overrides) as loaded at
/// startup.
#[derive(Debug, serde::Deserialize)]
pub struct ServerConfig {
    /// Address the HTTP listener binds to.
    #[serde(default = "default_listen_address")]
    pub listen_address: String,
    /// Port the HTTP listener binds to.
    #[serde(default = "default_port")]
    pub port: u16,

    /// `[search]` section.
    #[serde(default)]
    pub search: SearchConfig,
    /// `[weather]` section.
    #[serde(default)]
    pub weather: WeatherConfig,
    /// `[hofvarpnir]` section.
    #[serde(default)]
    pub hofvarpnir: HofvarpnirConfig,
    /// `[[health_checks]]` entries.
    #[serde(default)]
    pub health_checks: Vec<HealthCheckConfig>,
    /// `[[quick_links]]` entries.
    #[serde(default)]
    pub quick_links: Vec<QuickLinkConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_address: default_listen_address(),
            port: default_port(),
            search: SearchConfig::default(),
            weather: WeatherConfig::default(),
            hofvarpnir: HofvarpnirConfig::default(),
            health_checks: Vec::new(),
            quick_links: Vec::new(),
        }
    }
}

/// `[search]` config: which web-search engine the header box submits to.
#[derive(Debug, Default, serde::Deserialize)]
pub struct SearchConfig {
    /// Engine kind (TOML `type`).
    #[serde(default, rename = "type")]
    pub engine: SearchEngineType,
    /// Base URL of the instance; search is disabled while this is `None`.
    pub url: Option<String>,
}

/// Configured search-engine kind (TOML `type = "searxng"`). Extend alongside
/// [`SearchEngineKind`] as new engines are supported.
#[derive(Debug, Default, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchEngineType {
    /// A self-hosted `SearXNG` instance.
    #[default]
    Searxng,
}

impl SearchConfig {
    /// Build the app-level [`SearchEngine`] (search is disabled when no `url`).
    #[must_use]
    pub fn engine(&self) -> Option<SearchEngine> {
        let kind = match self.engine {
            SearchEngineType::Searxng => SearchEngineKind::Searxng,
        };
        self.url.clone().map(|url| SearchEngine { kind, url })
    }
}

/// `[weather]` config: the location the weather card reports on.
#[derive(Debug, serde::Deserialize)]
pub struct WeatherConfig {
    /// Latitude in decimal degrees.
    #[serde(default = "default_latitude")]
    pub latitude: f64,
    /// Longitude in decimal degrees.
    #[serde(default = "default_longitude")]
    pub longitude: f64,
    /// Display name for the location.
    #[serde(default = "default_location_name")]
    pub location: String,
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

/// `[hofvarpnir]` config: the download-archive instance the video card reads.
#[derive(Debug, Default, serde::Deserialize)]
pub struct HofvarpnirConfig {
    /// Base URL of the instance.
    pub url: Option<String>,
    /// Bearer token, if the instance requires one.
    pub api_key: Option<String>,
}

/// `[[quick_links]]` config: a static shortcut shown in the Quick Links card.
#[derive(Debug, serde::Deserialize)]
pub struct QuickLinkConfig {
    /// Display label.
    pub name: String,
    /// Destination URL.
    pub url: String,
    /// Optional leading icon (inline glyph or remote image URL).
    #[serde(default)]
    pub icon: Option<String>,
}

/// `[[health_checks]]` config: one endpoint probed by the health card.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct HealthCheckConfig {
    /// Display label.
    pub name: String,
    /// Endpoint to probe.
    pub url: String,
    /// Per-probe timeout in milliseconds.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    /// Status code that counts as healthy (any 2xx when `None`).
    pub expected_status: Option<u16>,
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

/// Load the server config from the default locations plus `DASHBOARD__*` env
/// vars.
///
/// # Errors
///
/// Returns an error if a config file is present but unreadable/malformed, or if
/// the merged values don't deserialize into [`ServerConfig`].
pub fn load_config() -> Result<ServerConfig> {
    load_config_from(&DEFAULT_CONFIG_PATHS)
}

/// Load the server config from `paths` (each optional, ascending precedence)
/// plus `DASHBOARD__*` env vars.
///
/// Paths are passed to `config::File::with_name`, so the extension is optional
/// and may be omitted. Tests use this to point at fixtures instead of the
/// developer's real, secret-bearing `config.toml`.
///
/// # Errors
///
/// Returns an error if a config file is present but unreadable/malformed, or if
/// the merged values don't deserialize into [`ServerConfig`].
pub fn load_config_from(paths: &[&str]) -> Result<ServerConfig> {
    use config::{Config, Environment, File};

    let mut builder = Config::builder()
        .set_default("listen_address", default_listen_address())?
        .set_default("port", i64::from(default_port()))?;

    for path in paths {
        builder = builder.add_source(File::with_name(path).required(false));
    }

    let config = builder
        .add_source(
            Environment::with_prefix("DASHBOARD")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    Ok(config.try_deserialize()?)
}

/// Push service URLs and secrets into the environment for the fetchers to read.
pub fn apply_config_to_env(config: &ServerConfig) {
    /// Set an env var (the `config` crate already validated these values).
    fn set(key: &str, value: &str) {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    set("WEATHER_LAT", &config.weather.latitude.to_string());
    set("WEATHER_LON", &config.weather.longitude.to_string());
    set("WEATHER_LOCATION", &config.weather.location);

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

#[cfg(test)]
// The config floats are exact literals round-tripped through TOML/serde, so
// `assert_eq!` on them is comparing the same bit pattern, not doing arithmetic.
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// Absolute path to a fixture under `tests/fixtures`, extension omitted so
    /// the `config` crate resolves it. Absolute so the test doesn't depend on
    /// the test binary's working directory.
    fn fixture(stem: &str) -> String {
        format!("{}/tests/fixtures/{stem}", env!("CARGO_MANIFEST_DIR"))
    }

    // ------------------------------------------------------------------
    // load_config_from
    // ------------------------------------------------------------------

    #[test]
    fn load_config_from_no_sources_yields_defaults() {
        let cfg = load_config_from(&[]).expect("defaults must load");

        assert_eq!(cfg.listen_address, "0.0.0.0");
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.weather.latitude, 52.52);
        assert_eq!(cfg.weather.longitude, 13.41);
        assert_eq!(cfg.weather.location, "Berlin");
        assert!(cfg.search.url.is_none());
        assert!(cfg.hofvarpnir.url.is_none());
        assert!(cfg.hofvarpnir.api_key.is_none());
        assert!(cfg.health_checks.is_empty());
        assert!(cfg.quick_links.is_empty());
    }

    #[test]
    fn load_config_from_missing_file_is_not_an_error() {
        let cfg = load_config_from(&[&fixture("does_not_exist")]).expect("optional source");
        assert_eq!(cfg.port, 8080);
    }

    #[test]
    fn load_config_from_fixture_parses_every_section() {
        let path = fixture("server_config");
        let cfg = load_config_from(&[&path]).expect("fixture must load");

        assert_eq!(cfg.listen_address, "127.0.0.1");
        assert_eq!(cfg.port, 9123);

        assert_eq!(
            cfg.search.url.as_deref(),
            Some("https://search.example.com")
        );
        assert!(matches!(cfg.search.engine, SearchEngineType::Searxng));

        assert_eq!(cfg.weather.latitude, 48.14);
        assert_eq!(cfg.weather.longitude, 11.58);
        assert_eq!(cfg.weather.location, "Munich");

        assert_eq!(
            cfg.hofvarpnir.url.as_deref(),
            Some("http://hof.example:8000")
        );
        assert_eq!(cfg.hofvarpnir.api_key.as_deref(), Some("fixture-key"));

        assert_eq!(cfg.health_checks.len(), 2);
        assert_eq!(cfg.health_checks[0].name, "Grafana");
        assert_eq!(
            cfg.health_checks[0].url,
            "http://grafana.example/api/health"
        );
        assert_eq!(cfg.health_checks[0].timeout_ms, 1500);
        assert_eq!(cfg.health_checks[0].expected_status, Some(204));
        // Second entry omits both optional keys, exercising the serde defaults.
        assert_eq!(cfg.health_checks[1].name, "Prometheus");
        assert_eq!(cfg.health_checks[1].timeout_ms, 5000);
        assert_eq!(cfg.health_checks[1].expected_status, None);

        assert_eq!(cfg.quick_links.len(), 2);
        assert_eq!(cfg.quick_links[0].name, "Grafana");
        assert_eq!(cfg.quick_links[0].url, "http://grafana.example");
        assert_eq!(cfg.quick_links[0].icon.as_deref(), Some("📊"));
        assert_eq!(cfg.quick_links[1].name, "Router");
        assert_eq!(cfg.quick_links[1].icon, None);
    }

    // ------------------------------------------------------------------
    // SearchConfig::engine
    // ------------------------------------------------------------------

    #[test]
    fn search_engine_is_none_without_url() {
        let cfg = SearchConfig::default();
        assert!(cfg.engine().is_none());
    }

    #[test]
    fn search_engine_is_some_with_url() {
        let cfg = SearchConfig {
            engine: SearchEngineType::Searxng,
            url: Some("https://search.example.com/".to_string()),
        };

        let engine = cfg.engine().expect("url set");
        assert_eq!(engine.kind, SearchEngineKind::Searxng);
        assert_eq!(engine.url, "https://search.example.com/");
        assert_eq!(engine.action_url(), "https://search.example.com/search");
    }

    // ------------------------------------------------------------------
    // Drift guards
    // ------------------------------------------------------------------

    /// The server's weather defaults and the app's `DashboardConfig` defaults
    /// are written out independently; keep them from drifting apart.
    #[test]
    fn server_weather_defaults_match_dashboard_config_defaults() {
        let server = ServerConfig::default();
        let app = DashboardConfig::default();

        assert_eq!(server.weather.latitude, app.latitude);
        assert_eq!(server.weather.longitude, app.longitude);
        assert_eq!(server.weather.location, app.location_name);
    }

    /// A card id has to be registered in three places (`CARD_IDS`, the
    /// `render_card` match, and `views::CARDS`). Adding it to only some of them
    /// silently drops the card from either the SSE stream or the page.
    #[test]
    fn card_ids_and_views_cards_agree() {
        let mut sse_ids: Vec<&str> = CARD_IDS.to_vec();
        let mut display_ids: Vec<&str> = views::CARDS.iter().map(|(id, _, _)| *id).collect();

        // The two lists are intentionally ordered differently (SSE push order
        // vs. display order), so compare them as sets.
        sse_ids.sort_unstable();
        display_ids.sort_unstable();
        assert_eq!(sse_ids, display_ids);
    }

    #[tokio::test]
    async fn render_card_rejects_unknown_ids() {
        let cfg = DashboardConfig::default();
        assert!(render_card(&cfg, "bogus").await.is_none());
        // Ids are matched case-sensitively.
        assert!(render_card(&cfg, "Weather").await.is_none());
        assert!(render_card(&cfg, "").await.is_none());
    }

    /// `links` is the only card that renders without touching the network, so
    /// it is the one `render_card` arm this test can drive end to end.
    #[tokio::test]
    async fn render_card_renders_the_offline_links_card() {
        let cfg = DashboardConfig::default();
        let markup = render_card(&cfg, "links").await.expect("known id");
        assert!(markup.into_string().contains("No quick links configured"));
    }

    // ------------------------------------------------------------------
    // apply_config_to_env
    // ------------------------------------------------------------------

    /// Every assertion about `apply_config_to_env` lives in this single test.
    /// The function writes process-global env vars via `unsafe set_var`, so
    /// splitting it into parallel tests would race.
    #[test]
    fn apply_config_to_env_sets_only_configured_values() {
        const KEYS: [&str; 6] = [
            "WEATHER_LAT",
            "WEATHER_LON",
            "WEATHER_LOCATION",
            "HOFVARPNIR_URL",
            "HOFVARPNIR_API_KEY",
            "HEALTH_CHECKS",
        ];

        // Snapshot whatever the ambient environment had so it can be restored.
        let saved: Vec<(&str, Option<String>)> =
            KEYS.iter().map(|k| (*k, std::env::var(k).ok())).collect();

        for key in KEYS {
            unsafe { std::env::remove_var(key) };
        }

        // 1. Bare defaults: weather is always written, the optional values are not.
        apply_config_to_env(&ServerConfig::default());
        assert_eq!(std::env::var("WEATHER_LAT").as_deref(), Ok("52.52"));
        assert_eq!(std::env::var("WEATHER_LON").as_deref(), Ok("13.41"));
        assert_eq!(std::env::var("WEATHER_LOCATION").as_deref(), Ok("Berlin"));
        assert!(std::env::var("HOFVARPNIR_URL").is_err());
        assert!(std::env::var("HOFVARPNIR_API_KEY").is_err());
        assert!(
            std::env::var("HEALTH_CHECKS").is_err(),
            "an empty health_checks list must not set HEALTH_CHECKS"
        );

        // 2. A url but no api key: only HOFVARPNIR_URL appears.
        let cfg = ServerConfig {
            weather: WeatherConfig {
                latitude: 1.5,
                longitude: -2.25,
                location: "Nowhere".to_string(),
            },
            hofvarpnir: HofvarpnirConfig {
                url: Some("http://hof.example".to_string()),
                api_key: None,
            },
            ..ServerConfig::default()
        };
        apply_config_to_env(&cfg);
        assert_eq!(std::env::var("WEATHER_LAT").as_deref(), Ok("1.5"));
        assert_eq!(std::env::var("WEATHER_LON").as_deref(), Ok("-2.25"));
        assert_eq!(std::env::var("WEATHER_LOCATION").as_deref(), Ok("Nowhere"));
        assert_eq!(
            std::env::var("HOFVARPNIR_URL").as_deref(),
            Ok("http://hof.example")
        );
        assert!(std::env::var("HOFVARPNIR_API_KEY").is_err());
        assert!(std::env::var("HEALTH_CHECKS").is_err());

        // 3. Api key and a non-empty health-check list: both are written, and
        //    HEALTH_CHECKS round-trips through the fetcher's JSON shape.
        let cfg = ServerConfig {
            hofvarpnir: HofvarpnirConfig {
                url: Some("http://hof.example".to_string()),
                api_key: Some("secret".to_string()),
            },
            health_checks: vec![HealthCheckConfig {
                name: "Grafana".to_string(),
                url: "http://grafana.example/api/health".to_string(),
                timeout_ms: 1500,
                expected_status: Some(204),
            }],
            ..ServerConfig::default()
        };
        apply_config_to_env(&cfg);
        assert_eq!(std::env::var("HOFVARPNIR_API_KEY").as_deref(), Ok("secret"));
        let json = std::env::var("HEALTH_CHECKS").expect("non-empty list sets HEALTH_CHECKS");
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed[0]["name"], "Grafana");
        assert_eq!(parsed[0]["timeout_ms"], 1500);
        assert_eq!(parsed[0]["expected_status"], 204);

        // 4. Already-set values are never cleared, only overwritten.
        apply_config_to_env(&ServerConfig::default());
        assert!(
            std::env::var("HEALTH_CHECKS").is_ok(),
            "an empty list leaves a previously written HEALTH_CHECKS in place"
        );
        assert!(std::env::var("HOFVARPNIR_URL").is_ok());

        // Restore the ambient environment.
        for (key, value) in saved {
            match value {
                Some(v) => unsafe { std::env::set_var(key, v) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}
