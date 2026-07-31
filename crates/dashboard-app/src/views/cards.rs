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

/// Map a wind bearing to an 8-point compass label.
///
/// Bearings are normalised into `0..360` first, so out-of-range values from the
/// API still yield a label instead of an empty string.
const fn wind_direction_to_cardinal(degrees: u16) -> &'static str {
    match degrees % 360 {
        0..=22 | 338..=359 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        _ => "NW",
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
    // Anything that isn't `Healthy` counts against the banner. Keying off
    // `unhealthy_count` alone would report "All systems operational" for an
    // overview made up entirely of `Degraded` or `Unknown` checks, since
    // `summarize` counts those in neither bucket.
    let needs_attention = overview.checks.len().saturating_sub(overview.healthy_count);
    let all_healthy = needs_attention == 0;
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
                    span { (needs_attention) " service(s) need attention" }
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

#[cfg(test)]
mod tests {
    use super::{
        health_body, health_placeholder_body, quick_links_body, video_body, weather_body,
        wind_direction_to_cardinal,
    };
    use crate::types::{
        HealthCheck, HealthOverview, HealthStatus, HofvarpnirStatus, HofvarpnirVideo,
        WeatherCondition, WeatherData,
    };
    use crate::{Icon, QuickLink};
    use chrono::{DateTime, Utc};
    use ulid::Ulid;

    /// 2023-11-14 22:13:20 UTC — fixed so the rendered `%H:%M` strings are stable.
    fn ts() -> DateTime<Utc> {
        DateTime::from_timestamp(1_700_000_000, 0).expect("valid unix timestamp")
    }

    fn weather(wind_direction: u16) -> WeatherData {
        WeatherData {
            temperature: 12.34,
            feels_like: 10.06,
            humidity: 71,
            wind_speed: 5.0,
            wind_direction,
            condition: WeatherCondition::Rain,
            icon: "🌧️".to_owned(),
            location: "Berlin".to_owned(),
            fetched_at: ts(),
        }
    }

    fn video(id: &str) -> HofvarpnirVideo {
        HofvarpnirVideo {
            id: id.to_owned(),
            title: "A Video".to_owned(),
            platform_video_id: "dQw4w9WgXcQ".to_owned(),
            platform: "youtube".to_owned(),
            source_name: Some("Some Channel".to_owned()),
            thumbnail_url: Some("https://cdn.example.com/thumb.jpg".to_owned()),
            downloaded_at: Some(ts()),
            published_at: Some(ts()),
        }
    }

    fn status(videos: Vec<HofvarpnirVideo>, total: u64) -> HofvarpnirStatus {
        HofvarpnirStatus {
            videos,
            total_downloads: total,
            fetched_at: ts(),
        }
    }

    fn check(name: &str, status: HealthStatus, response_time_ms: Option<u32>) -> HealthCheck {
        HealthCheck {
            id: Ulid::nil(),
            name: name.to_owned(),
            url: "https://svc.example.com/health".to_owned(),
            status,
            response_time_ms,
            last_checked: ts(),
            error_message: None,
        }
    }

    fn overview(checks: Vec<HealthCheck>, healthy: usize, unhealthy: usize) -> HealthOverview {
        HealthOverview {
            checks,
            healthy_count: healthy,
            unhealthy_count: unhealthy,
            fetched_at: ts(),
        }
    }

    // -------------------------------------------- wind_direction_to_cardinal

    #[test]
    fn wind_direction_to_cardinal_maps_all_sixteen_boundaries() {
        let cases: [(u16, &str); 18] = [
            (0, "N"),
            (22, "N"),
            (23, "NE"),
            (67, "NE"),
            (68, "E"),
            (112, "E"),
            (113, "SE"),
            (157, "SE"),
            (158, "S"),
            (202, "S"),
            (203, "SW"),
            (247, "SW"),
            (248, "W"),
            (292, "W"),
            (293, "NW"),
            (337, "NW"),
            (338, "N"),
            (360, "N"),
        ];

        for (degrees, expected) in cases {
            assert_eq!(
                wind_direction_to_cardinal(degrees),
                expected,
                "{degrees} degrees"
            );
        }
    }

    #[test]
    fn wind_direction_to_cardinal_normalizes_out_of_range_bearings() {
        // These used to fall into a `_ => ""` catch-all, rendering a bare
        // trailing space after the wind speed. `% 360` folds them back onto the
        // compass: 361 -> 1 (N), 400 -> 40 (NE), 65535 -> 15 (N).
        for (degrees, expected) in [(360_u16, "N"), (361, "N"), (400, "NE"), (65_535, "N")] {
            assert_eq!(
                wind_direction_to_cardinal(degrees),
                expected,
                "{degrees} degrees"
            );
        }
    }

    // -------------------------------------------------------- weather_body

    #[test]
    fn weather_body_renders_the_current_conditions() {
        let s = weather_body(&weather(180)).into_string();

        assert!(s.contains("🌧️"), "{s}");
        assert!(s.contains(">12.3<"), "{s}");
        assert!(s.contains("10.1°C"), "{s}");
        assert!(s.contains("71%"), "{s}");
        assert!(s.contains("5.0 km/h S"), "{s}");
        assert!(s.contains("Berlin"), "{s}");
        assert!(s.contains("Updated: 22:13"), "{s}");
    }

    #[test]
    fn weather_body_labels_an_out_of_range_wind_bearing() {
        // 400 degrees normalizes to 40 -> NE. Before the `% 360` fix this
        // rendered "5.0 km/h " with a dangling trailing space.
        let s = weather_body(&weather(400)).into_string();
        assert!(s.contains("5.0 km/h NE"), "{s}");
    }

    // ---------------------------------------------------------- video_body

    #[test]
    fn video_body_renders_the_empty_state() {
        let s = video_body(&status(Vec::new(), 0)).into_string();

        assert!(s.contains("No recent downloads"), "{s}");
        assert!(!s.contains("<ul"), "{s}");
        assert!(s.contains("videos archived"), "{s}");
    }

    #[test]
    fn video_body_renders_a_video_row() {
        let s = video_body(&status(vec![video("01H")], 42)).into_string();

        assert!(s.contains("A Video"), "{s}");
        assert!(s.contains("Some Channel"), "{s}");
        assert!(
            s.contains(r#"src="https://cdn.example.com/thumb.jpg""#),
            "{s}"
        );
        assert!(s.contains("Uploaded "), "{s}");
        assert!(s.contains("Archived "), "{s}");
        assert!(s.contains("Nov 14, 22:13"), "{s}");
        assert!(s.contains(">42<"), "{s}");
    }

    #[test]
    fn video_body_falls_back_to_the_platform_without_a_source_name() {
        let mut v = video("01H");
        v.source_name = None;
        v.platform = "peertube".to_owned();
        let s = video_body(&status(vec![v], 1)).into_string();

        assert!(s.contains("peertube"), "{s}");
        assert!(!s.contains("text-accent-cyan"), "{s}");
    }

    #[test]
    fn video_body_omits_the_uploaded_row_without_a_publish_date() {
        let mut v = video("01H");
        v.published_at = None;
        let s = video_body(&status(vec![v], 1)).into_string();

        assert!(!s.contains("Uploaded "), "{s}");
        assert!(s.contains("Archived "), "{s}");
    }

    #[test]
    fn video_body_renders_unknown_without_a_download_date() {
        let mut v = video("01H");
        v.downloaded_at = None;
        let s = video_body(&status(vec![v], 1)).into_string();

        assert!(s.contains("Archived <span"), "{s}");
        assert!(s.contains(">Unknown<"), "{s}");
    }

    #[test]
    fn video_body_uses_a_youtube_thumbnail_for_non_youtube_platforms() {
        // BUG: the thumbnail fallback hardcodes an i.ytimg.com URL regardless of
        // `platform`, so a Vimeo download points at a YouTube CDN path.
        let mut v = video("01H");
        v.platform = "vimeo".to_owned();
        v.thumbnail_url = None;
        v.platform_video_id = "123456789".to_owned();
        let s = video_body(&status(vec![v], 1)).into_string();

        assert!(
            s.contains("https://i.ytimg.com/vi/123456789/mqdefault.jpg"),
            "{s}"
        );
    }

    #[test]
    fn video_body_renders_one_row_per_video() {
        let videos = vec![video("01A"), video("01B"), video("01C")];
        let s = video_body(&status(videos, 3)).into_string();

        assert_eq!(s.matches(r#"<li class="flex gap-4">"#).count(), 3, "{s}");
    }

    // --------------------------------------------------------- health_body

    #[test]
    fn health_body_reports_all_systems_operational_when_nothing_is_unhealthy() {
        let checks = vec![check("Grafana", HealthStatus::Healthy, Some(12))];
        let s = health_body(&overview(checks, 1, 0)).into_string();

        assert!(s.contains("All systems operational"), "{s}");
        assert!(s.contains("text-accent-green"), "{s}");
        assert!(s.contains("12ms"), "{s}");
        assert!(s.contains("Checked: 22:13:20"), "{s}");
    }

    #[test]
    fn health_body_reports_the_unhealthy_count() {
        let checks = vec![
            check("Grafana", HealthStatus::Healthy, Some(12)),
            check("Jellyfin", HealthStatus::Unhealthy, None),
        ];
        let s = health_body(&overview(checks, 1, 1)).into_string();

        assert!(s.contains("1 service(s) need attention"), "{s}");
        assert!(s.contains("⚠️"), "{s}");
        assert!(!s.contains("All systems operational"), "{s}");
    }

    #[test]
    fn health_body_renders_an_em_dash_without_a_response_time() {
        let checks = vec![check("Jellyfin", HealthStatus::Unhealthy, None)];
        let s = health_body(&overview(checks, 0, 1)).into_string();

        assert!(s.contains(">—</span>"), "{s}");
        assert!(!s.contains("ms</span>"), "{s}");
    }

    #[test]
    fn health_body_renders_a_status_marker_per_check_state() {
        let checks = vec![
            check("A", HealthStatus::Healthy, Some(1)),
            check("B", HealthStatus::Degraded, Some(2)),
            check("C", HealthStatus::Unhealthy, Some(3)),
            check("D", HealthStatus::Unknown, Some(4)),
        ];
        let s = health_body(&overview(checks, 1, 1)).into_string();

        for marker in [
            "border-l-accent-green",
            "border-l-accent-yellow",
            "border-l-accent-red",
            "border-l-text-muted",
        ] {
            assert!(s.contains(marker), "missing {marker} in {s}");
        }
    }

    #[test]
    fn health_body_renders_an_error_tooltip_when_present() {
        let mut c = check("Jellyfin", HealthStatus::Unhealthy, None);
        c.error_message = Some("Timeout after 5000ms".to_owned());
        let s = health_body(&overview(vec![c], 0, 1)).into_string();

        assert!(s.contains(r#"title="Timeout after 5000ms""#), "{s}");
        assert!(s.contains("ℹ️"), "{s}");
    }

    #[test]
    fn health_body_flags_an_all_degraded_overview() {
        // `summarize` counts Degraded in neither bucket, so this overview has
        // `unhealthy_count == 0`. The banner keys off `checks.len() -
        // healthy_count` instead, or a fully degraded fleet would read as
        // "All systems operational".
        let checks = vec![
            check("A", HealthStatus::Degraded, Some(1)),
            check("B", HealthStatus::Degraded, Some(2)),
        ];
        let s = health_body(&overview(checks, 0, 0)).into_string();

        assert!(s.contains("2 service(s) need attention"), "{s}");
        assert!(s.contains("text-accent-red"), "{s}");
        assert!(!s.contains("All systems operational"), "{s}");
    }

    // --------------------------------------------- health_placeholder_body

    #[test]
    fn health_placeholder_body_renders_a_neutral_row_per_name() {
        let names = vec!["Grafana".to_owned(), "Jellyfin".to_owned()];
        let s = health_placeholder_body(&names).into_string();

        assert!(s.contains("Checking services…"), "{s}");
        assert!(s.contains("Grafana"), "{s}");
        assert!(s.contains("Jellyfin"), "{s}");
        assert_eq!(s.matches("animate-pulse").count(), 2, "{s}");
        assert_eq!(s.matches(">—</span>").count(), 2, "{s}");
    }

    #[test]
    fn health_placeholder_body_renders_the_banner_without_names() {
        let s = health_placeholder_body(&[]).into_string();

        assert!(s.contains("Checking services…"), "{s}");
        assert!(!s.contains("animate-pulse"), "{s}");
    }

    // ---------------------------------------------------- quick_links_body

    #[test]
    fn quick_links_body_renders_the_empty_state() {
        let s = quick_links_body(&[]).into_string();

        assert!(s.contains("No quick links configured"), "{s}");
        assert!(!s.contains("<a "), "{s}");
    }

    #[test]
    fn quick_links_body_renders_a_text_icon_as_a_glyph_span() {
        let links = vec![QuickLink {
            name: "Grafana".to_owned(),
            url: "https://grafana.example.com".to_owned(),
            icon: Some(Icon::Text("📊".to_owned())),
        }];
        let s = quick_links_body(&links).into_string();

        assert!(
            s.contains(r#"<span class="text-base shrink-0">📊</span>"#),
            "{s}"
        );
        assert!(!s.contains("<img"), "{s}");
        assert!(s.contains(r#"href="https://grafana.example.com""#), "{s}");
        assert!(s.contains("Grafana"), "{s}");
    }

    #[test]
    fn quick_links_body_renders_an_image_icon_as_an_img_tag() {
        let links = vec![QuickLink {
            name: "Jellyfin".to_owned(),
            url: "https://jellyfin.example.com".to_owned(),
            icon: Some(Icon::Image(
                "https://cdn.example.com/jellyfin.svg".to_owned(),
            )),
        }];
        let s = quick_links_body(&links).into_string();

        assert!(
            s.contains(r#"src="https://cdn.example.com/jellyfin.svg""#),
            "{s}"
        );
        assert!(s.contains(r#"loading="lazy""#), "{s}");
        assert!(!s.contains(r#"class="text-base shrink-0""#), "{s}");
    }

    #[test]
    fn quick_links_body_omits_the_icon_slot_when_unset() {
        let links = vec![QuickLink {
            name: "Plain".to_owned(),
            url: "https://plain.example.com".to_owned(),
            icon: None,
        }];
        let s = quick_links_body(&links).into_string();

        assert!(!s.contains("<img"), "{s}");
        assert!(!s.contains(r#"class="text-base shrink-0""#), "{s}");
        assert!(s.contains("Plain"), "{s}");
    }

    #[test]
    fn quick_links_body_renders_one_anchor_per_link() {
        let links: Vec<QuickLink> = (0..3)
            .map(|i| QuickLink {
                name: format!("Link {i}"),
                url: format!("https://example.com/{i}"),
                icon: None,
            })
            .collect();
        let s = quick_links_body(&links).into_string();

        assert_eq!(s.matches("<a href=").count(), 3, "{s}");
    }
}
