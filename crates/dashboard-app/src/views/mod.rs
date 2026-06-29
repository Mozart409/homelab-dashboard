//! Maud views: the page shell, shared card chrome, and per-card partials.
//!
//! Each card is a self-contained partial (header + body) that is delivered two
//! ways: pushed over the shared `/events` SSE stream (swapped by event name)
//! and served directly from `/card/{id}` for manual refresh. The [`page`]
//! shell renders the same chrome with skeleton bodies so the layout is stable
//! before the first SSE frame arrives.

mod cards;
mod search;

pub use cards::{
    health_card, homeassistant_card, jellyfin_card, proxmox_card, video_card, weather_card,
};
pub use search::{search_error, search_results};

use crate::DashboardConfig;
use maud::{DOCTYPE, Markup, PreEscaped, html};

/// htmx 2.x core (pinned with SRI per the user's snippet).
const HTMX_SRC: &str = "https://cdn.jsdelivr.net/npm/htmx.org@2.0.10/dist/htmx.min.js";
const HTMX_INTEGRITY: &str =
    "sha384-H5SrcfygHmAuTDZphMHqBJLc3FhssKjG7w/CeCpFReSfwBWDTKpkzPP8c+cLsK+V";
/// htmx Server-Sent Events extension.
const HTMX_SSE_SRC: &str = "https://cdn.jsdelivr.net/npm/htmx-ext-sse@2.2.2/sse.js";

/// All cards rendered on the dashboard, in display order.
/// `(id, title, icon)` — `id` doubles as the SSE event name and `/card/{id}` route.
pub const CARDS: [(&str, &str, Option<&str>); 6] = [
    ("weather", "Weather", None),
    ("video", "Recent Downloads", None),
    ("proxmox", "Proxmox", Some("🖥️")),
    ("jellyfin", "Jellyfin", Some("🎬")),
    ("homeassistant", "Home Assistant", Some("🏠")),
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
                link rel="stylesheet" href="/dashboard.css";
                script src=(HTMX_SRC) integrity=(HTMX_INTEGRITY) crossorigin="anonymous" {}
                script src=(HTMX_SSE_SRC) crossorigin="anonymous" {}
            }
            body {
                main class="max-w-[1400px] mx-auto p-6 min-h-screen flex flex-col bg-bg-primary text-text-primary font-sans" {
                    header class="flex items-center justify-between gap-6 mb-8 pb-6 border-b border-border flex-col md:flex-row" {
                        h1 class="font-mono text-2xl font-semibold text-text-primary flex items-center gap-2" { "🏠 Homelab" }
                        div class="flex-1 max-w-[500px] w-full md:w-auto" {
                            (search_box(cfg.searxng_url.is_some()))
                        }
                    }

                    div hx-ext="sse" sse-connect="/events" class="flex-1 flex flex-col gap-8" {
                        // Top row: Weather + Recent videos
                        section class="grid gap-6 grid-cols-1 lg:grid-cols-[300px_1fr]" {
                            (card_shell("weather", "Weather", None))
                            (card_shell("video", "Recent Downloads", None))
                        }

                        // Middle row: Service cards
                        section class="grid gap-6 grid-cols-1 md:grid-cols-2 xl:grid-cols-3" {
                            (card_shell("proxmox", "Proxmox", Some("🖥️")))
                            (card_shell("jellyfin", "Jellyfin", Some("🎬")))
                            (card_shell("homeassistant", "Home Assistant", Some("🏠")))
                        }

                        // Bottom row: Health checks
                        section class="grid gap-6" {
                            (card_shell("health", "Service Health", Some("🩺")))
                        }
                    }

                    footer class="mt-auto pt-6 border-t border-border-subtle flex items-center justify-center gap-2 text-text-muted text-sm" {
                        span { "Homelab Dashboard" }
                        span { "•" }
                        span { "Powered by Axum + Maud + htmx" }
                    }
                }
            }
        }
    }
}

/// The outer card container: holds the `.card` chrome, subscribes to its SSE
/// event, and starts with a skeleton body. htmx swaps `innerHTML` on each frame.
fn card_shell(id: &str, title: &str, icon: Option<&str>) -> Markup {
    html! {
        div id=(format!("card-{id}"))
            class="bg-bg-card border border-border rounded-xl overflow-hidden"
            sse-swap=(id)
            hx-swap="innerHTML"
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

/// Search box wired to the `/search` endpoint via htmx.
fn search_box(enabled: bool) -> Markup {
    html! {
        div class="relative" {
            div class="relative flex items-center" {
                svg class="absolute left-4 w-[18px] h-[18px] text-text-muted pointer-events-none" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" {
                    circle cx="11" cy="11" r="8" {}
                    path d="m21 21-4.35-4.35" {}
                }
                input
                    type="text"
                    name="q"
                    autocomplete="off"
                    class="w-full py-2 px-4 pl-[calc(1rem+24px)] bg-bg-secondary border border-border rounded-lg text-text-primary font-mono text-sm transition-all duration-150 placeholder:text-text-muted focus:outline-none focus:border-accent-blue focus:ring-[3px] focus:ring-accent-blue/15"
                    placeholder=(if enabled { "Search the web..." } else { "Search disabled (set searxng.url)" })
                    disabled[!enabled]
                    hx-get="/search"
                    hx-trigger="input changed delay:300ms, keyup[key=='Enter']"
                    hx-target="#search-results"
                    hx-indicator="#search-spinner";
                div id="search-spinner" class="htmx-indicator absolute right-4 w-4 h-4 border-2 border-border border-t-accent-blue rounded-full animate-spin" {}
            }
            div id="search-results" class="absolute top-[calc(100%+0.25rem)] left-0 right-0 z-[100]" {}
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
