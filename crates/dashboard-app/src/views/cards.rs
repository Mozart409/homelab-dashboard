//! Per-card partials: fetch the service data and render header + body.
//!
//! Each function returns the inner HTML for its card container (the same markup
//! pushed over SSE and served from `/card/{id}`). Fetch errors render an inline
//! error body rather than failing the request.

use maud::{Markup, html};

use super::{card, card_error};
use crate::fetch::{get_health_overview, get_hofvarpnir_status, get_weather};
use crate::types::{HealthOverview, HealthStatus, HofvarpnirStatus, WeatherData};
use crate::{DashboardConfig, Icon, QuickLink};

// ============================================================================
// Weather
// ============================================================================

/// Weather card partial.
pub async fn weather_card(cfg: &DashboardConfig) -> Markup {
    match get_weather(cfg.latitude, cfg.longitude, cfg.location_name.clone()).await {
        Ok(weather) => card("weather", "Weather", None, weather_body(&weather)),
        Err(e) => card_error("weather", "Weather", None, &e.to_string()),
    }
}

fn weather_body(weather: &WeatherData) -> Markup {
    let wind = wind_direction_to_cardinal(weather.wind_direction);
    html! {
        div class="p-4" {
            div class="flex items-center gap-4 mb-4" {
                span class="text-5xl" { (weather.icon) }
                div class="flex items-start" {
                    span class="font-mono text-4xl font-semibold leading-none" { (format!("{:.1}", weather.temperature)) }
                    span class="text-base text-text-muted mt-1" { "°C" }
                }
            }
            div class="text-sm text-text-secondary mb-4" { (weather.location) }
            div class="flex flex-col gap-1 pt-4 border-t border-border-subtle" {
                div class="flex justify-between text-xs" {
                    span class="text-text-muted" { "Feels like" }
                    span class="text-text-secondary font-mono" { (format!("{:.1}°C", weather.feels_like)) }
                }
                div class="flex justify-between text-xs" {
                    span class="text-text-muted" { "Humidity" }
                    span class="text-text-secondary font-mono" { (format!("{}%", weather.humidity)) }
                }
                div class="flex justify-between text-xs" {
                    span class="text-text-muted" { "Wind" }
                    span class="text-text-secondary font-mono" { (format!("{:.1} km/h {wind}", weather.wind_speed)) }
                }
            }
            div class="mt-4 text-xs text-text-muted" {
                "Updated: " (weather.fetched_at.format("%H:%M").to_string())
            }
        }
    }
}

const fn wind_direction_to_cardinal(degrees: u16) -> &'static str {
    match degrees {
        0..=22 | 338..=360 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        _ => "",
    }
}

// ============================================================================
// Recent downloads (Hofvarpnir)
// ============================================================================

/// Recent downloads card partial.
pub async fn video_card() -> Markup {
    match get_hofvarpnir_status(Some(6)).await {
        Ok(status) => card("video", "Recent Downloads", None, video_body(&status)),
        Err(e) => card_error("video", "Recent Downloads", None, &e.to_string()),
    }
}

fn video_body(status: &HofvarpnirStatus) -> Markup {
    html! {
        div class="p-4" {
            @if status.videos.is_empty() {
                div class="text-center py-4 text-text-muted text-sm" { "No recent downloads" }
            } @else {
                // Full-width card: 3 per row, so the 6 fetched videos fill two
                // rows. Collapses to 2 then 1 column on narrower viewports.
                ul class="grid gap-4 grid-cols-1 md:grid-cols-2 xl:grid-cols-3" {
                    @for video in &status.videos {
                        @let thumbnail = video.thumbnail_url.clone().unwrap_or_else(|| {
                            format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", video.platform_video_id)
                        });
                        @let fmt_ts = |dt: chrono::DateTime<chrono::Utc>| dt.format("%b %d, %H:%M").to_string();
                        @let uploaded = video.published_at.map(fmt_ts);
                        @let archived = video.downloaded_at.map_or_else(
                            || "Unknown".to_string(),
                            fmt_ts,
                        );
                        li class="flex gap-4" {
                            div class="relative shrink-0 w-[120px] h-[68px] bg-bg-elevated rounded overflow-hidden" {
                                img class="w-full h-full object-cover" src=(thumbnail) alt="" loading="lazy";
                                div class="absolute inset-0 flex items-center justify-center text-text-muted pointer-events-none" {
                                    svg class="w-6 h-6 opacity-30" viewBox="0 0 24 24" fill="currentColor" {
                                        path d="M8 5v14l11-7z" {}
                                    }
                                }
                            }
                            div class="flex-1 min-w-0" {
                                div class="text-sm font-medium text-text-primary truncate mb-1" title=(video.title) { (video.title) }
                                // The channel/playlist (or its custom name) is the
                                // useful label, so it gets the accent. Without one,
                                // the platform stands in.
                                @match video.source_name.as_deref() {
                                    Some(name) => div class="text-sm font-semibold text-accent-cyan truncate mb-1" title=(name) { (name) },
                                    None => div class="text-xs text-text-secondary mb-1" { (video.platform) },
                                }
                                // Upload date first, then when we archived it. Both
                                // labels are 8 characters, so the dates line up in
                                // the mono face without explicit columns. `Uploaded`
                                // is omitted when the API has no publish date.
                                div class="text-xs font-mono text-text-secondary leading-relaxed" {
                                    @if let Some(uploaded) = uploaded {
                                        div { "Uploaded " span class="text-text-primary" { (uploaded) } }
                                    }
                                    div { "Archived " span class="text-text-primary" { (archived) } }
                                }
                            }
                        }
                    }
                }
            }
            // Stat, not a table row: the count leads at full contrast with its
            // label trailing it. `justify-between` used to fling the number to
            // the far edge of a now full-width card, stranding it from the text
            // it belongs to.
            div class="flex items-baseline gap-2 mt-4 pt-4 border-t border-border-subtle" {
                span class="font-mono text-lg font-semibold text-text-primary" { (status.total_downloads) }
                span class="text-xs text-text-muted" { "videos archived" }
            }
        }
    }
}

// ============================================================================
// Health checks
// ============================================================================

/// Service health card partial.
pub async fn health_card() -> Markup {
    match get_health_overview().await {
        Ok(overview) => card(
            "health",
            "Service Health",
            Some("🩺"),
            health_body(&overview),
        ),
        Err(e) => card_error("health", "Service Health", Some("🩺"), &e.to_string()),
    }
}

/// Health card in its initial, pre-probe state: renders the configured service
/// names with a neutral "checking" status. The first SSE frame replaces this
/// with live statuses once the probe round completes.
#[must_use]
pub fn health_placeholder_card(names: &[String]) -> Markup {
    card(
        "health",
        "Service Health",
        Some("🩺"),
        health_placeholder_body(names),
    )
}

fn health_placeholder_body(names: &[String]) -> Markup {
    html! {
        div class="p-4" {
            div class="flex items-center gap-2 px-4 py-2 rounded mb-4 text-sm font-medium bg-bg-elevated text-text-muted" {
                span { "⚪" }
                span { "Checking services…" }
            }

            div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2" {
                @for name in names {
                    div class="flex flex-col gap-1 px-4 py-2 bg-bg-elevated rounded border-l-[3px] border-l-text-muted animate-pulse" {
                        div class="flex items-center gap-2" {
                            span class="text-xs" { "⚪" }
                            span class="text-sm font-medium" { (name) }
                        }
                        div class="flex items-center gap-2" {
                            span class="font-mono text-xs text-text-muted" { "—" }
                        }
                    }
                }
            }
        }
    }
}

fn health_body(overview: &HealthOverview) -> Markup {
    let all_healthy = overview.unhealthy_count == 0;
    html! {
        div class="p-4" {
            div class=(format!(
                "flex items-center gap-2 px-4 py-2 rounded mb-4 text-sm font-medium {}",
                if all_healthy { "bg-accent-green/10 text-accent-green" } else { "bg-accent-red/10 text-accent-red" }
            )) {
                @if all_healthy {
                    span { "✅" }
                    span { "All systems operational" }
                } @else {
                    span { "⚠️" }
                    span { (overview.unhealthy_count) " service(s) unhealthy" }
                }
            }

            div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2" {
                @for check in &overview.checks {
                    @let (border, icon) = match check.status {
                        HealthStatus::Healthy => ("border-l-accent-green", "🟢"),
                        HealthStatus::Degraded => ("border-l-accent-yellow", "🟡"),
                        HealthStatus::Unhealthy => ("border-l-accent-red", "🔴"),
                        HealthStatus::Unknown => ("border-l-text-muted", "⚪"),
                    };
                    @let response_time = check.response_time_ms.map_or_else(
                        || "—".to_string(),
                        |ms| format!("{ms}ms"),
                    );
                    div class=(format!("flex flex-col gap-1 px-4 py-2 bg-bg-elevated rounded border-l-[3px] {border}")) {
                        div class="flex items-center gap-2" {
                            span class="text-xs" { (icon) }
                            span class="text-sm font-medium" { (check.name) }
                        }
                        div class="flex items-center gap-2" {
                            span class="font-mono text-xs text-text-muted" { (response_time) }
                            @if let Some(msg) = &check.error_message {
                                span class="cursor-help" title=(msg) { "ℹ️" }
                            }
                        }
                    }
                }
            }

            div class="text-[0.7rem] text-text-muted pt-2 mt-4 border-t border-border-subtle" {
                "Checked: " (overview.fetched_at.format("%H:%M:%S").to_string())
            }
        }
    }
}

// ============================================================================
// Quick links
// ============================================================================

/// Quick links card partial. Static, config-driven — no fetch, so no error path.
#[must_use]
pub fn quick_links_card(cfg: &DashboardConfig) -> Markup {
    card(
        "links",
        "Quick Links",
        Some("🔗"),
        quick_links_body(&cfg.quick_links),
    )
}

fn quick_links_body(links: &[QuickLink]) -> Markup {
    html! {
        div class="p-4" {
            @if links.is_empty() {
                div class="text-center py-4 text-text-muted text-sm" { "No quick links configured" }
            } @else {
                div class="grid grid-cols-[repeat(auto-fill,minmax(180px,1fr))] gap-2" {
                    @for link in links {
                        a href=(link.url) target="_blank" rel="noopener"
                            class="flex items-center gap-2 px-4 py-2 bg-bg-elevated rounded border-l-[3px] border-l-accent-blue text-text-primary no-underline transition-colors duration-150 hover:text-accent-blue"
                        {
                            @if let Some(icon) = &link.icon {
                                @match icon {
                                    Icon::Text(glyph) => { span class="text-base shrink-0" { (glyph) } }
                                    Icon::Image(src) => {
                                        img class="w-5 h-5 shrink-0 object-contain" src=(src) alt="" loading="lazy";
                                    }
                                }
                            }
                            span class="text-sm font-medium truncate" { (link.name) }
                        }
                    }
                }
            }
        }
    }
}
