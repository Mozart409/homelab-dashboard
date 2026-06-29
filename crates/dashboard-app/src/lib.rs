#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
//! Homelab Dashboard - shared application crate.
//!
//! A self-hosted dashboard for monitoring homelab services including:
//! - `SearXNG` search integration
//! - Weather from Open-Meteo
//! - Hofvarpnir video downloads
//! - Proxmox VM status
//! - Jellyfin media server
//! - Home Assistant entities
//! - Custom health checks
//!
//! This crate holds the shared [`types`], the service [`fetch`]ers, and the
//! Maud [`views`]. The Axum server in `dashboard-server` wires them together.

pub mod fetch;
pub mod types;
pub mod views;

/// Display configuration the request handlers need to drive the fetchers.
///
/// Service URLs and secrets are read from the environment by the individual
/// fetchers; this only carries the values that vary per render.
#[derive(Debug, Clone)]
pub struct DashboardConfig {
    /// Weather location latitude.
    pub latitude: f64,
    /// Weather location longitude.
    pub longitude: f64,
    /// Weather location display name.
    pub location_name: String,
    /// `SearXNG` instance URL (search is disabled when `None`).
    pub searxng_url: Option<String>,
    /// Home Assistant entity IDs to show (empty shows all).
    pub ha_entity_ids: Vec<String>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            latitude: 52.52,
            longitude: 13.41,
            location_name: "Berlin".to_string(),
            searxng_url: None,
            ha_entity_ids: Vec::new(),
        }
    }
}
