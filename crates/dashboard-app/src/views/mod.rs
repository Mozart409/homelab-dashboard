//! Maud views: the page shell, shared card chrome, and per-card partials.
//!
//! Each card is a self-contained partial (header + body) that is delivered two
//! ways: pushed over the shared `/events` SSE stream (swapped by event name)
//! and served directly from `/card/{id}` for manual refresh. The [`page`]
//! shell renders the same chrome with skeleton bodies so the layout is stable
//! before the first SSE frame arrives.

mod cards;

pub use cards::{health_card, health_placeholder_card, quick_links_card, video_card, weather_card};

use crate::{DashboardConfig, SearchEngine};
use maud::{DOCTYPE, Markup, PreEscaped, html};

/// htmx 2.x core (pinned with SRI per the user's snippet).
const HTMX_SRC: &str = "https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js";
const HTMX_INTEGRITY: &str =
    "sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V";
/// htmx Server-Sent Events extension.
const HTMX_SSE_SRC: &str = "https://cdn.jsdelivr.net/npm/htmx-ext-sse@2.2.2/sse.js";
/// idiomorph + its htmx extension: powers `hx-swap="morph:innerHTML"` so cards
/// are reconciled in place. Unchanged nodes (e.g. video thumbnails) are kept
/// rather than recreated, so the browser doesn't re-request their assets.
const IDIOMORPH_SRC: &str =
    "https://cdn.jsdelivr.net/npm/idiomorph@0.7.3/dist/idiomorph-ext.min.js";

/// Build metadata embedded at compile time (git hash + build time via `build.rs`).
const VERSION: &str = env!("CARGO_PKG_VERSION");
const GIT_HASH: &str = env!("GIT_HASH");
const BUILD_TIME: &str = env!("BUILD_TIME");

/// Cache-busting token for the compiled stylesheet, derived from its size and
/// mtime. Deliberately *not* `GIT_HASH`: that only moves on commit, so every
/// `just css` / `css-watch` rebuild kept serving the browser its cached copy
/// under an unchanged `?v=`. Stat-per-render is cheap and always correct.
fn css_version() -> String {
    let dir = std::env::var("DASHBOARD_STATIC_DIR").unwrap_or_else(|_| "static".to_owned());
    std::fs::metadata(std::path::Path::new(&dir).join("dashboard.css"))
        .ok()
        .and_then(|meta| {
            let mtime = meta
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?;
            Some(format!("{:x}-{:x}", meta.len(), mtime.as_secs()))
        })
        .unwrap_or_else(|| GIT_HASH.to_owned())
}

/// All cards rendered on the dashboard, in display order.
/// `(id, title, icon)` — `id` doubles as the SSE event name and `/card/{id}` route.
pub const CARDS: [(&str, &str, Option<&str>); 4] = [
    ("links", "Quick Links", Some("🔗")),
    ("weather", "Weather", None),
    ("video", "Recent Downloads", None),
    ("health", "Service Health", Some("🩺")),
];

/// Full dashboard document.
#[must_use]
pub fn page(cfg: &DashboardConfig) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { "Homelab Dashboard" }
                link rel="icon" href="https://fav.farm/🏠";
                link rel="stylesheet" href=(format!("/dashboard.css?v={}", css_version()));
                script src=(HTMX_SRC) integrity=(HTMX_INTEGRITY) crossorigin="anonymous" {}
                script src=(HTMX_SSE_SRC) crossorigin="anonymous" {}
                script src=(IDIOMORPH_SRC) crossorigin="anonymous" {}
            }
            body {
                main class="max-w-[1400px] mx-auto p-6 min-h-screen flex flex-col bg-bg-primary text-text-primary font-sans" {
                    header class="flex items-center justify-between gap-6 mb-8 pb-6 border-b border-border flex-col md:flex-row" {
                        h1 class="font-mono text-2xl font-semibold text-text-primary flex items-center gap-2" { "🏠 Homelab" }
                        div class="flex-1 max-w-[500px] w-full md:w-auto" {
                            (search_box(cfg.search.as_ref()))
                        }
                    }

                    div hx-ext="sse,morph" sse-connect="/events" class="flex-1 flex flex-col gap-8" {
                        // Top row: Quick links — static config, so render the real
                        // body at load time; the SSE frame morphs identically over it.
                        section class="grid gap-6" {
                            div id="card-links"
                                class="bg-bg-card border border-border rounded-xl overflow-hidden"
                                sse-swap="links"
                                hx-swap="morph:innerHTML"
                            {
                                (quick_links_card(cfg))
                            }
                        }

                        // Recent downloads, full width (3 per row × 2).
                        section class="grid gap-6" {
                            (card_shell("video", "Recent Downloads", None))
                        }

                        // Bottom row: Weather + Health checks. Grid items stretch
                        // by default, so both cards share the height of the taller
                        // one. Health names are static config, so render neutral
                        // rows now; the SSE frame colors them once the first probe
                        // round completes.
                        section class="grid gap-6 grid-cols-1 lg:grid-cols-[300px_1fr]" {
                            (card_shell("weather", "Weather", None))
                            div id="card-health"
                                class="bg-bg-card border border-border rounded-xl overflow-hidden"
                                sse-swap="health"
                                hx-swap="morph:innerHTML"
                            {
                                (health_placeholder_card(&cfg.health_check_names))
                            }
                        }
                    }

                    footer class="mt-auto pt-6 border-t border-border-subtle flex flex-wrap items-center justify-center gap-2 text-text-muted text-sm" {
                        span { "Homelab Dashboard" }
                        span { "•" }
                        span { "Powered by Axum + Maud + htmx" }
                        span { "•" }
                        a href="https://github.com/Mozart409/homelab-dashboard" target="_blank" rel="noopener"
                            class="text-text-muted no-underline transition-colors duration-150 hover:text-text-primary" {
                            "GitHub"
                        }
                        span { "•" }
                        span class="font-mono text-xs" {
                            "v" (VERSION) " · " (GIT_HASH) " · " (BUILD_TIME)
                        }
                    }
                }
            }
        }
    }
}

/// The outer card container: holds the `.card` chrome, subscribes to its SSE
/// event, and starts with a skeleton body. Each frame is morphed into place so
/// unchanged nodes (e.g. thumbnails) are preserved instead of recreated.
fn card_shell(id: &str, title: &str, icon: Option<&str>) -> Markup {
    html! {
        div id=(format!("card-{id}"))
            class="bg-bg-card border border-border rounded-xl overflow-hidden"
            sse-swap=(id)
            hx-swap="morph:innerHTML"
        {
            (card_header(id, title, icon))
            (skeleton(4))
        }
    }
}

/// Card header with title and a manual-refresh button (hx-get the partial).
#[must_use]
pub fn card_header(id: &str, title: &str, icon: Option<&str>) -> Markup {
    html! {
        div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary" {
            h3 class="font-mono text-sm font-semibold text-text-primary flex items-center gap-2" {
                @if let Some(icon) = icon {
                    span class="text-lg" { (icon) }
                }
                (title)
            }
            button
                class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                hx-get=(format!("/card/{id}"))
                hx-target=(format!("#card-{id}"))
                hx-swap="innerHTML"
                title="Refresh"
            {
                (refresh_icon())
            }
        }
    }
}

/// Render a full card partial: header + the supplied body.
#[must_use]
#[allow(clippy::needless_pass_by_value)]
pub fn card(id: &str, title: &str, icon: Option<&str>, body: Markup) -> Markup {
    html! {
        (card_header(id, title, icon))
        (body)
    }
}

/// Render a card whose fetch failed.
#[must_use]
pub fn card_error(id: &str, title: &str, icon: Option<&str>, message: &str) -> Markup {
    card(
        id,
        title,
        icon,
        html! {
            div class="flex items-center gap-2 p-4 text-accent-red text-sm" {
                span { "⚠️" }
                span { (message) }
            }
        },
    )
}

/// A generic pulsing skeleton body with `rows` placeholder lines.
#[must_use]
pub fn skeleton(rows: usize) -> Markup {
    html! {
        div class="p-4 animate-pulse" {
            @for _ in 0..rows {
                div class="w-full h-8 bg-bg-elevated rounded mb-2" {}
            }
        }
    }
}

/// Header search box: a plain GET form that submits straight to the configured
/// engine (e.g. `SearXNG`), so the browser is redirected there with the query.
/// Renders a disabled placeholder when no engine is configured.
fn search_box(engine: Option<&SearchEngine>) -> Markup {
    html! {
        @if let Some(engine) = engine {
            form action=(engine.action_url()) method="get" target="_blank" rel="noopener" role="search" class="relative flex items-center" {
                svg class="absolute left-4 w-[18px] h-[18px] text-text-muted pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                    circle cx="11" cy="11" r="8" {}
                    path d="m21 21-4.35-4.35" {}
                }
                input
                    type="search"
                    name=(engine.query_param())
                    autocomplete="off"
                    autofocus
                    aria-label="Search"
                    class="w-full py-2 px-4 pl-[calc(1rem+24px)] bg-bg-secondary border border-border rounded-lg text-text-primary font-mono text-sm transition-all duration-150 placeholder:text-text-muted focus:outline-none focus:border-accent-blue focus:ring-[3px] focus:ring-accent-blue/15"
                    placeholder="Search the web...";
            }
        } @else {
            div class="relative flex items-center" {
                svg class="absolute left-4 w-[18px] h-[18px] text-text-muted pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                    circle cx="11" cy="11" r="8" {}
                    path d="m21 21-4.35-4.35" {}
                }
                input
                    type="search"
                    disabled
                    class="w-full py-2 px-4 pl-[calc(1rem+24px)] bg-bg-secondary border border-border rounded-lg text-text-muted font-mono text-sm"
                    placeholder="Search disabled (set [search] url)";
            }
        }
    }
}

/// The shared refresh (circular arrows) icon.
#[must_use]
pub fn refresh_icon() -> Markup {
    PreEscaped(
        r#"<svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/><path d="M16 21h5v-5"/></svg>"#
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::{
        CARDS, DashboardConfig, SearchEngine, card, card_error, card_header, page, search_box,
        skeleton,
    };
    use crate::SearchEngineKind;
    use maud::html;

    /// The one row `skeleton` repeats; counting it counts the rows.
    const SKELETON_ROW: &str = r#"<div class="w-full h-8 bg-bg-elevated rounded mb-2"></div>"#;

    fn searxng() -> SearchEngine {
        SearchEngine {
            kind: SearchEngineKind::Searxng,
            url: "https://s.io".to_owned(),
        }
    }

    // --------------------------------------------------------- card_header

    #[test]
    fn card_header_renders_icon_when_present() {
        let s = card_header("health", "Service Health", Some("🩺")).into_string();
        assert!(s.contains(r#"<span class="text-lg">🩺</span>"#), "{s}");
        assert!(s.contains("Service Health"), "{s}");
    }

    #[test]
    fn card_header_omits_icon_span_when_none() {
        let s = card_header("weather", "Weather", None).into_string();
        assert!(!s.contains(r#"class="text-lg""#), "{s}");
        assert!(s.contains("Weather"), "{s}");
    }

    #[test]
    fn card_header_wires_htmx_refresh_attributes() {
        let s = card_header("weather", "Weather", None).into_string();
        assert!(s.contains(r#"hx-get="/card/weather""#), "{s}");
        assert!(s.contains(r##"hx-target="#card-weather""##), "{s}");
        assert!(s.contains(r#"hx-swap="innerHTML""#), "{s}");
    }

    #[test]
    fn card_header_escapes_id_and_title() {
        let id = r#"a"><script>"#;
        let title = r#"T"<script>alert(1)</script>"#;
        let s = card_header(id, title, None).into_string();

        assert!(!s.contains("<script>"), "unescaped <script> in {s}");
        assert!(!s.contains("</script>"), "unescaped </script> in {s}");
        // The injected quote must not terminate the hx-get attribute early.
        assert!(!s.contains(r#"hx-get="/card/a">"#), "{s}");
        assert!(s.contains("&lt;script&gt;"), "{s}");
        assert!(s.contains("&quot;"), "{s}");
    }

    // ------------------------------------------------------- card / errors

    #[test]
    fn card_renders_header_then_body() {
        let s = card(
            "links",
            "Quick Links",
            Some("🔗"),
            html! { p { "body-marker" } },
        )
        .into_string();

        let header = s.find("Quick Links").expect("header rendered");
        let body = s.find("body-marker").expect("body rendered");
        assert!(header < body, "body should follow the header in {s}");
    }

    #[test]
    fn card_error_renders_message_with_warning_glyph() {
        let s = card_error("weather", "Weather", None, "Failed to fetch weather").into_string();
        assert!(s.contains("⚠️"), "{s}");
        assert!(s.contains("Failed to fetch weather"), "{s}");
        assert!(s.contains("text-accent-red"), "{s}");
    }

    #[test]
    fn card_error_escapes_the_message() {
        // Error strings come from reqwest / remote responses, so they are untrusted.
        let s = card_error(
            "video",
            "Recent Downloads",
            None,
            r#"<img src=x onerror="alert(1)">"#,
        )
        .into_string();

        assert!(!s.contains("<img src=x"), "unescaped markup in {s}");
        assert!(s.contains("&lt;img src=x"), "{s}");
    }

    // ------------------------------------------------------------ skeleton

    #[test]
    fn skeleton_emits_exactly_the_requested_row_count() {
        for rows in [0_usize, 1, 4, 64] {
            let s = skeleton(rows).into_string();
            assert_eq!(s.matches(SKELETON_ROW).count(), rows, "rows = {rows}");
            assert!(s.contains("animate-pulse"), "rows = {rows}");
        }
    }

    // ---------------------------------------------------------- search_box

    #[test]
    fn search_box_renders_a_get_form_for_a_configured_engine() {
        let engine = searxng();
        let s = search_box(Some(&engine)).into_string();

        assert!(s.contains(r#"action="https://s.io/search""#), "{s}");
        assert!(s.contains(r#"method="get""#), "{s}");
        assert!(s.contains(r#"name="q""#), "{s}");
        assert!(s.contains(r#"role="search""#), "{s}");
        assert!(!s.contains("disabled"), "{s}");
    }

    #[test]
    fn search_box_renders_a_disabled_input_without_an_engine() {
        let s = search_box(None).into_string();

        assert!(s.contains("disabled"), "{s}");
        assert!(s.contains("Search disabled"), "{s}");
        assert!(!s.contains("<form"), "{s}");
    }

    // ---------------------------------------------------------------- page

    #[test]
    fn page_renders_the_search_form_when_an_engine_is_configured() {
        let cfg = DashboardConfig {
            search: Some(searxng()),
            ..DashboardConfig::default()
        };
        let s = page(&cfg).into_string();

        assert!(s.contains(r#"action="https://s.io/search""#), "{s}");
        assert!(!s.contains("Search disabled"), "{s}");
    }

    #[test]
    fn page_renders_the_disabled_search_box_without_an_engine() {
        let s = page(&DashboardConfig::default()).into_string();

        assert!(s.contains("Search disabled"), "{s}");
        assert!(!s.contains("<form"), "{s}");
    }

    #[test]
    fn page_contains_every_card_shell() {
        let s = page(&DashboardConfig::default()).into_string();

        for (id, _, _) in CARDS {
            assert!(
                s.contains(&format!(r#"id="card-{id}""#)),
                "missing card-{id}"
            );
            assert!(
                s.contains(&format!(r#"sse-swap="{id}""#)),
                "missing sse {id}"
            );
        }
    }

    #[test]
    fn page_renders_the_document_shell() {
        let s = page(&DashboardConfig::default()).into_string();

        assert!(s.starts_with("<!DOCTYPE html>"), "{s}");
        assert!(s.contains("<title>Homelab Dashboard</title>"), "{s}");
        assert!(s.contains(r#"sse-connect="/events""#), "{s}");
    }

    #[test]
    fn page_renders_the_health_placeholder_names() {
        let cfg = DashboardConfig {
            health_check_names: vec!["Grafana".to_owned(), "Jellyfin".to_owned()],
            ..DashboardConfig::default()
        };
        let s = page(&cfg).into_string();

        assert!(s.contains("Checking services…"), "{s}");
        assert!(s.contains("Grafana"), "{s}");
        assert!(s.contains("Jellyfin"), "{s}");
    }

    #[test]
    fn page_renders_the_quick_links_empty_state_by_default() {
        let s = page(&DashboardConfig::default()).into_string();
        assert!(s.contains("No quick links configured"), "{s}");
    }
}
