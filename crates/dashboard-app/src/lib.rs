#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::missing_errors_doc)]
//! Homelab Dashboard - shared application crate.
//!
//! A self-hosted dashboard for monitoring homelab services including:
//! - `SearXNG` search integration
//! - Weather from Open-Meteo
//! - Hofvarpnir video downloads
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
    /// Web-search engine the header box submits to (search is hidden when `None`).
    pub search: Option<SearchEngine>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            latitude: 52.52,
            longitude: 13.41,
            location_name: "Berlin".to_string(),
            search: None,
        }
    }
}

/// Web-search engine the header search box submits queries to.
///
/// The box is a plain GET form that the browser sends straight to the
/// instance, so the search happens on the engine itself — the dashboard never
/// proxies it. Only `SearXNG` is supported today; the [`kind`](Self::kind)
/// discriminator leaves room for more engines later.
#[derive(Debug, Clone)]
pub struct SearchEngine {
    /// Which engine the [`url`](Self::url) points at.
    pub kind: SearchEngineKind,
    /// Base URL of the instance, e.g. `https://search.example.com`.
    pub url: String,
}

/// Supported search engines. Extend this as new engines are wired up.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchEngineKind {
    /// A self-hosted `SearXNG` instance (`{url}/search?q=…`).
    #[default]
    Searxng,
}

impl SearchEngine {
    /// The form `action` URL the browser GETs the query against.
    #[must_use]
    pub fn action_url(&self) -> String {
        let base = self.url.trim_end_matches('/');
        match self.kind {
            SearchEngineKind::Searxng => format!("{base}/search"),
        }
    }

    /// The query-string parameter the engine reads the search term from.
    #[must_use]
    pub const fn query_param(&self) -> &'static str {
        match self.kind {
            SearchEngineKind::Searxng => "q",
        }
    }
}
