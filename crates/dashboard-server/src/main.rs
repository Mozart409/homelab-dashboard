#![allow(clippy::multiple_crate_versions)]
//! Homelab Dashboard Server binary.
//!
//! Process entry point only: install `color-eyre`, set up tracing, load the
//! config, push secrets into the environment for the fetchers, build the router
//! from [`dashboard_server::build_router`] and serve it. Everything worth
//! testing lives in the library crate next door (`src/lib.rs`).

use std::net::SocketAddr;

use color_eyre::eyre::Result;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use dashboard_app::{DashboardConfig, Icon, QuickLink};
use dashboard_server::{AppState, apply_config_to_env, build_router, load_config};

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
            search: config.search.engine(),
            quick_links: config
                .quick_links
                .iter()
                .map(|l| QuickLink {
                    name: l.name.clone(),
                    url: l.url.clone(),
                    icon: l.icon.clone().map(Icon::from_config),
                })
                .collect(),
            health_check_names: config
                .health_checks
                .iter()
                .map(|c| c.name.clone())
                .collect(),
        },
    };

    // Static assets (compiled dashboard.css, etc.). Defaults to ./static for
    // local dev; the Nix package points this at its share directory.
    let static_dir = std::env::var("DASHBOARD_STATIC_DIR").unwrap_or_else(|_| "static".to_string());

    let app = build_router(state, &static_dir);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Dashboard available at http://{}", addr);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
