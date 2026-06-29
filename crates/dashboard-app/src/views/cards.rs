//! Per-card partials: fetch the service data and render header + body.
//!
//! Each function returns the inner HTML for its card container (the same markup
//! pushed over SSE and served from `/card/{id}`). Fetch errors render an inline
//! error body rather than failing the request.

use maud::{Markup, html};

use super::{card, card_error};
use crate::DashboardConfig;
use crate::fetch::{
    get_health_overview, get_hofvarpnir_status, get_homeassistant_status, get_jellyfin_status,
    get_proxmox_status, get_weather,
};
use crate::types::{
    HealthOverview, HealthStatus, HofvarpnirStatus, HomeAssistantStatus, JellyfinItemType,
    JellyfinStatus, NodeStatus, ProxmoxStatus, VmStatus, WeatherData,
};

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
    match get_hofvarpnir_status(Some(3)).await {
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
                ul class="flex flex-col gap-4" {
                    @for video in &status.videos {
                        @let thumbnail = video.thumbnail_url.clone().unwrap_or_else(|| {
                            format!("https://i.ytimg.com/vi/{}/mqdefault.jpg", video.platform_video_id)
                        });
                        @let downloaded = video.downloaded_at.map_or_else(
                            || "Unknown".to_string(),
                            |dt| dt.format("%b %d, %H:%M").to_string(),
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
                                div class="text-xs text-text-secondary mb-1" { (video.platform) }
                                div class="text-[0.7rem] text-text-muted font-mono" { (downloaded) }
                            }
                        }
                    }
                }
            }
            div class="flex justify-between pt-4 border-t border-border-subtle text-xs" {
                span class="text-text-muted" { "Total downloads:" }
                span class="text-text-secondary font-mono" { (status.total_downloads) }
            }
        }
    }
}

// ============================================================================
// Proxmox
// ============================================================================

/// Proxmox card partial.
pub async fn proxmox_card() -> Markup {
    match get_proxmox_status().await {
        Ok(status) => card("proxmox", "Proxmox", Some("🖥️"), proxmox_body(&status)),
        Err(e) => card_error("proxmox", "Proxmox", Some("🖥️"), &e.to_string()),
    }
}

fn proxmox_body(status: &ProxmoxStatus) -> Markup {
    let running_vms = status
        .vms
        .iter()
        .filter(|vm| vm.status == VmStatus::Running)
        .count();
    let total_vms = status.vms.len();

    html! {
        div class="p-4" {
            div class="mb-4" {
                h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2" { "Nodes" }
                div class="flex flex-col gap-2" {
                    @for node in &status.nodes {
                        @let border = match node.status {
                            NodeStatus::Online => "border-l-accent-green",
                            NodeStatus::Offline => "border-l-accent-red",
                            NodeStatus::Unknown => "border-l-text-muted",
                        };
                        @let mem_percent = mem_percent(node.memory_used, node.memory_total);
                        div class=(format!("flex justify-between items-center p-2 bg-bg-elevated rounded border-l-[3px] {border}")) {
                            span class="font-mono text-sm font-medium" { (node.name) }
                            div class="flex gap-4" {
                                span class="font-mono text-xs text-text-secondary" { (format!("CPU: {:.0}%", node.cpu_usage * 100.0)) }
                                span class="font-mono text-xs text-text-secondary" { (format!("RAM: {mem_percent}%")) }
                            }
                        }
                    }
                }
            }

            div class="mb-4" {
                h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2" { "Virtual Machines" }
                div class="flex gap-6" {
                    div class="flex flex-col items-center" {
                        span class="font-mono text-2xl font-semibold text-accent-green" { (running_vms) }
                        span class="text-xs text-text-muted" { "Running" }
                    }
                    div class="flex flex-col items-center" {
                        span class="font-mono text-2xl font-semibold text-text-secondary" { (total_vms) }
                        span class="text-xs text-text-muted" { "Total" }
                    }
                }
            }

            div class="text-[0.7rem] text-text-muted pt-2 border-t border-border-subtle" {
                "Updated: " (status.fetched_at.format("%H:%M:%S").to_string())
            }
        }
    }
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
fn mem_percent(used: u64, total: u64) -> u8 {
    if total > 0 {
        (used as f64 / total as f64 * 100.0) as u8
    } else {
        0
    }
}

// ============================================================================
// Jellyfin
// ============================================================================

/// Jellyfin card partial.
pub async fn jellyfin_card() -> Markup {
    match get_jellyfin_status().await {
        Ok(status) => card("jellyfin", "Jellyfin", Some("🎬"), jellyfin_body(&status)),
        Err(e) => card_error("jellyfin", "Jellyfin", Some("🎬"), &e.to_string()),
    }
}

fn jellyfin_body(status: &JellyfinStatus) -> Markup {
    let items: Vec<_> = status.recently_added.iter().take(3).collect();
    html! {
        div class="p-4" {
            div class="flex justify-between items-center mb-4" {
                span class="font-medium" { (status.server_name) }
                span class="font-mono text-xs text-text-muted" { "v" (status.version) }
            }

            div class="flex gap-6 mb-4" {
                div class="flex flex-col items-center gap-1" {
                    span class="text-xl" { "📺" }
                    span class="font-mono text-xl font-semibold" { (status.active_streams) }
                    span class="text-[0.7rem] text-text-muted" { "Streaming" }
                }
                div class="flex flex-col items-center gap-1" {
                    span class="text-xl" { "🎥" }
                    span class="font-mono text-xl font-semibold" { (status.total_movies) }
                    span class="text-[0.7rem] text-text-muted" { "Movies" }
                }
                div class="flex flex-col items-center gap-1" {
                    span class="text-xl" { "📺" }
                    span class="font-mono text-xl font-semibold" { (status.total_series) }
                    span class="text-[0.7rem] text-text-muted" { "Series" }
                }
            }

            @if !items.is_empty() {
                div class="mb-4" {
                    h4 class="text-xs font-semibold text-text-muted uppercase tracking-wide mb-2" { "Recently Added" }
                    ul class="flex flex-col gap-1" {
                        @for item in &items {
                            @let display_name = match (&item.item_type, &item.series_name) {
                                (JellyfinItemType::Episode, Some(series)) => format!("{series} - {}", item.name),
                                _ => item.name.clone(),
                            };
                            @let icon = match item.item_type {
                                JellyfinItemType::Movie => "🎬",
                                JellyfinItemType::Episode => "📺",
                                _ => "📁",
                            };
                            li class="flex items-center gap-2 py-1" {
                                span class="shrink-0" { (icon) }
                                span class="text-xs text-text-secondary truncate" title=(display_name) { (display_name) }
                            }
                        }
                    }
                }
            }

            div class="text-[0.7rem] text-text-muted pt-2 border-t border-border-subtle" {
                "Updated: " (status.fetched_at.format("%H:%M:%S").to_string())
            }
        }
    }
}

// ============================================================================
// Home Assistant
// ============================================================================

/// Home Assistant card partial.
pub async fn homeassistant_card(cfg: &DashboardConfig) -> Markup {
    match get_homeassistant_status(cfg.ha_entity_ids.clone()).await {
        Ok(status) => card(
            "homeassistant",
            "Home Assistant",
            Some("🏠"),
            homeassistant_body(&status),
        ),
        Err(e) => card_error(
            "homeassistant",
            "Home Assistant",
            Some("🏠"),
            &e.to_string(),
        ),
    }
}

fn homeassistant_body(status: &HomeAssistantStatus) -> Markup {
    html! {
        div class="p-4" {
            div class="text-xs text-text-muted mb-4" { "Home Assistant " (status.version) }

            div class="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-2" {
                @for entity in &status.entities {
                    @let border = match entity.state.to_lowercase().as_str() {
                        "on" | "home" | "open" | "playing" => "border-l-accent-green",
                        "unavailable" | "unknown" => "border-l-accent-red opacity-60",
                        _ => "border-l-text-muted",
                    };
                    @let value = entity.unit.as_ref().map_or_else(
                        || entity.state.clone(),
                        |unit| format!("{} {unit}", entity.state),
                    );
                    div class=(format!("flex items-center gap-2 p-2 bg-bg-elevated rounded border-l-[3px] {border}")) {
                        span class="text-xl" { (entity.icon.clone().unwrap_or_else(|| "📊".to_string())) }
                        div class="flex-1 min-w-0" {
                            span class="block text-xs text-text-muted truncate" { (entity.friendly_name) }
                            span class="block font-mono text-sm font-medium" { (value) }
                        }
                    }
                }
            }

            div class="text-[0.7rem] text-text-muted pt-2 mt-4 border-t border-border-subtle" {
                "Updated: " (status.fetched_at.format("%H:%M:%S").to_string())
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
