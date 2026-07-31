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
    /// Static shortcuts rendered in the Quick Links card.
    pub quick_links: Vec<QuickLink>,
    /// Configured service-health names, in probe order. Used to render the
    /// health card's neutral placeholder rows before the first probe completes;
    /// the live statuses arrive over SSE.
    pub health_check_names: Vec<String>,
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            latitude: 52.52,
            longitude: 13.41,
            location_name: "Berlin".to_string(),
            search: None,
            quick_links: Vec::new(),
            health_check_names: Vec::new(),
        }
    }
}

/// A labelled shortcut shown in the Quick Links card.
///
/// Purely static, configured under `[[quick_links]]` in `config.toml`; unlike
/// health checks the dashboard never contacts the URL, it just links to it.
#[derive(Debug, Clone)]
pub struct QuickLink {
    /// Display label, e.g. `Grafana`.
    pub name: String,
    /// Destination URL the link points at.
    pub url: String,
    /// Optional leading icon (inline glyph or remote image).
    pub icon: Option<Icon>,
}

/// A quick-link icon, either an inline glyph or a remote image.
///
/// The `[[quick_links]]` config carries a single `icon` string; it's classified
/// into one of these variants at load time (see [`Icon::from_config`]) so the
/// view can render each kind without re-sniffing the string.
#[derive(Debug, Clone)]
pub enum Icon {
    /// A short inline glyph rendered as text, e.g. an emoji like `📊`.
    Text(String),
    /// A remote image URL (e.g. an SVG from dashboard-icons), rendered as `<img>`.
    Image(String),
}

impl Icon {
    /// Classify a raw config string: anything with an `http(s)` scheme is an
    /// [`Image`](Self::Image), everything else is inline [`Text`](Self::Text).
    #[must_use]
    pub fn from_config(raw: String) -> Self {
        if raw.starts_with("http://") || raw.starts_with("https://") {
            Self::Image(raw)
        } else {
            Self::Text(raw)
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

#[cfg(test)]
mod tests {
    // `DashboardConfig::default` stores exact literals, so comparing them back
    // with `==` is the assertion, not an approximation.
    #![allow(clippy::float_cmp)]

    use super::{DashboardConfig, Icon, SearchEngine, SearchEngineKind};

    fn searxng(url: &str) -> SearchEngine {
        SearchEngine {
            kind: SearchEngineKind::Searxng,
            url: url.to_owned(),
        }
    }

    // ---------------------------------------------------------------- Icon

    #[test]
    fn from_config_classifies_http_urls_as_image() {
        assert!(matches!(
            Icon::from_config("http://cdn.example.com/grafana.svg".to_owned()),
            Icon::Image(url) if url == "http://cdn.example.com/grafana.svg"
        ));
    }

    #[test]
    fn from_config_classifies_https_urls_as_image() {
        assert!(matches!(
            Icon::from_config("https://cdn.example.com/grafana.svg".to_owned()),
            Icon::Image(url) if url == "https://cdn.example.com/grafana.svg"
        ));
    }

    #[test]
    fn from_config_classifies_glyphs_as_text() {
        assert!(matches!(
            Icon::from_config("📊".to_owned()),
            Icon::Text(glyph) if glyph == "📊"
        ));
    }

    #[test]
    fn from_config_classifies_empty_string_as_text() {
        assert!(matches!(
            Icon::from_config(String::new()),
            Icon::Text(glyph) if glyph.is_empty()
        ));
    }

    #[test]
    fn from_config_scheme_match_is_case_sensitive() {
        // Uppercase schemes are legal URLs but fall through to the text branch.
        assert!(matches!(
            Icon::from_config("HTTPS://X".to_owned()),
            Icon::Text(glyph) if glyph == "HTTPS://X"
        ));
    }

    #[test]
    fn from_config_classifies_non_http_schemes_as_text() {
        for raw in ["ftp://x", "//cdn/x.svg", "httpsfoo"] {
            assert!(
                matches!(Icon::from_config(raw.to_owned()), Icon::Text(_)),
                "{raw} should classify as Text"
            );
        }
    }

    #[test]
    fn from_config_does_not_trim_leading_whitespace() {
        // No `.trim()` before the prefix check, so a stray space demotes a URL.
        assert!(matches!(
            Icon::from_config(" https://x".to_owned()),
            Icon::Text(glyph) if glyph == " https://x"
        ));
    }

    // -------------------------------------------------------- SearchEngine

    #[test]
    fn action_url_appends_search_path() {
        assert_eq!(searxng("https://s.io").action_url(), "https://s.io/search");
    }

    #[test]
    fn action_url_normalizes_trailing_slashes() {
        for base in ["https://s.io", "https://s.io/", "https://s.io///"] {
            assert_eq!(
                searxng(base).action_url(),
                "https://s.io/search",
                "base {base} should normalize to a single /search"
            );
        }
    }

    #[test]
    fn action_url_yields_exactly_one_search_segment() {
        for base in ["https://s.io", "https://s.io/", "https://s.io///", "", "/"] {
            let url = searxng(base).action_url();
            assert!(url.ends_with("/search"), "{base} -> {url}");
            assert!(!url.ends_with("//search"), "{base} -> {url}");
            assert_eq!(url.matches("/search").count(), 1, "{base} -> {url}");
        }
    }

    #[test]
    fn action_url_handles_empty_and_root_bases() {
        assert_eq!(searxng("").action_url(), "/search");
        assert_eq!(searxng("/").action_url(), "/search");
    }

    #[test]
    fn query_param_for_searxng_is_q() {
        assert_eq!(searxng("https://s.io").query_param(), "q");
    }

    // ----------------------------------------------------- DashboardConfig

    #[test]
    fn default_config_points_at_berlin() {
        let cfg = DashboardConfig::default();
        assert_eq!(cfg.latitude, 52.52);
        assert_eq!(cfg.longitude, 13.41);
        assert_eq!(cfg.location_name, "Berlin");
    }

    #[test]
    fn default_config_has_no_search_and_no_entries() {
        let cfg = DashboardConfig::default();
        assert!(cfg.search.is_none());
        assert!(cfg.quick_links.is_empty());
        assert!(cfg.health_check_names.is_empty());
    }
}
