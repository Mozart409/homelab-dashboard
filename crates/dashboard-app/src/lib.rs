#![allow(clippy::multiple_crate_versions)]
//! Homelab Dashboard - Leptos Application
//!
//! A self-hosted dashboard for monitoring homelab services including:
//! - `SearXNG` search integration
//! - Weather from Open-Meteo
//! - Hofvarpnir video downloads
//! - Proxmox VM status
//! - Jellyfin media server
//! - Home Assistant entities
//! - Custom health checks

pub mod app;
pub mod components;
pub mod server;
pub mod types;

pub use app::App;

/// Hydrate the application on the client side.
#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();

    leptos::mount::hydrate_body(App);
}
