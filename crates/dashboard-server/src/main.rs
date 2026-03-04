//! Homelab Dashboard Server
//!
//! Axum server that serves the Leptos application with SSR and hydration.

use axum::Router;
use color_eyre::eyre::Result;
use leptos::prelude::*;
use leptos_axum::{generate_route_list, LeptosRoutes};
use std::net::SocketAddr;
use tower_http::compression::CompressionLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use dashboard_app::App;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize error handling
    color_eyre::install()?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            "dashboard_server=debug,dashboard_app=debug,tower_http=debug".into()
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let config = load_config()?;
    
    // Log configuration (without secrets)
    tracing::info!(
        "Starting dashboard server on {}:{}",
        config.listen_address,
        config.port
    );

    // Set environment variables for server functions
    apply_config_to_env(&config);

    // Get Leptos configuration
    let leptos_options = LeptosOptions::builder()
        .output_name("dashboard")
        .site_root("target/site")
        .site_pkg_dir("pkg")
        .site_addr(SocketAddr::new(
            config.listen_address.parse()?,
            config.port,
        ))
        .build();

    // Generate routes from the app
    let routes = generate_route_list(App);

    // Build the Axum router
    let app = Router::new()
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .with_state(leptos_options);

    // Start the server
    let addr = SocketAddr::new(config.listen_address.parse()?, config.port);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    tracing::info!("Dashboard available at http://{}", addr);
    
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

/// HTML shell for SSR
#[allow(clippy::needless_pass_by_value)]
fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <AutoReload options=options.clone()/>
                <HydrationScripts options=options.clone()/>
            </head>
            <body>
                <App/>
            </body>
        </html>
    }
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
    pinchflat: PinchflatConfig,
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
}

#[derive(Debug, Default, serde::Deserialize)]
struct PinchflatConfig {
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

fn default_port() -> u16 {
    3000
}

fn default_latitude() -> f64 {
    52.52
}

fn default_longitude() -> f64 {
    13.41
}

fn default_location_name() -> String {
    "Berlin".to_string()
}

fn default_timeout() -> u64 {
    5000
}

fn load_config() -> Result<ServerConfig> {
    use config::{Config, Environment, File};

    let config = Config::builder()
        // Start with defaults
        .set_default("listen_address", "0.0.0.0")?
        .set_default("port", 3000)?
        // Load from config file if present
        .add_source(File::with_name("config").required(false))
        .add_source(File::with_name("/etc/homelab-dashboard/config").required(false))
        // Override with environment variables
        .add_source(
            Environment::with_prefix("DASHBOARD")
                .separator("__")
                .try_parsing(true),
        )
        .build()?;

    Ok(config.try_deserialize()?)
}

fn apply_config_to_env(config: &ServerConfig) {
    // SearXNG
    if let Some(url) = &config.searxng.url {
        unsafe { std::env::set_var("SEARXNG_URL", url); }
    }

    // Weather
    unsafe {
        std::env::set_var("WEATHER_LAT", config.weather.latitude.to_string());
        std::env::set_var("WEATHER_LON", config.weather.longitude.to_string());
        std::env::set_var("WEATHER_LOCATION", &config.weather.location);
    }

    // Proxmox
    if let Some(url) = &config.proxmox.url {
        unsafe { std::env::set_var("PROXMOX_URL", url); }
    }
    if let Some(token_id) = &config.proxmox.token_id {
        unsafe { std::env::set_var("PROXMOX_TOKEN_ID", token_id); }
    }
    if let Some(token_secret) = &config.proxmox.token_secret {
        unsafe { std::env::set_var("PROXMOX_TOKEN_SECRET", token_secret); }
    }

    // Jellyfin
    if let Some(url) = &config.jellyfin.url {
        unsafe { std::env::set_var("JELLYFIN_URL", url); }
    }
    if let Some(api_key) = &config.jellyfin.api_key {
        unsafe { std::env::set_var("JELLYFIN_API_KEY", api_key); }
    }

    // Home Assistant
    if let Some(url) = &config.homeassistant.url {
        unsafe { std::env::set_var("HOMEASSISTANT_URL", url); }
    }
    if let Some(token) = &config.homeassistant.token {
        unsafe { std::env::set_var("HOMEASSISTANT_TOKEN", token); }
    }

    // Pinchflat
    if let Some(url) = &config.pinchflat.url {
        unsafe { std::env::set_var("PINCHFLAT_URL", url); }
    }
    if let Some(api_key) = &config.pinchflat.api_key {
        unsafe { std::env::set_var("PINCHFLAT_API_KEY", api_key); }
    }

    // Health checks as JSON
    if !config.health_checks.is_empty()
        && let Ok(json) = serde_json::to_string(&config.health_checks)
    {
        unsafe { std::env::set_var("HEALTH_CHECKS", json); }
    }
}
