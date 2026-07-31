//! Shared types for the homelab dashboard.
//!
//! These types are used both on the server and client side,
//! serialized via serde for the server function boundary.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ulid::Ulid;

// ============================================================================
// Weather Types (Open-Meteo)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeatherData {
    pub temperature: f64,
    pub feels_like: f64,
    pub humidity: u8,
    pub wind_speed: f64,
    pub wind_direction: u16,
    pub condition: WeatherCondition,
    pub icon: String,
    pub location: String,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherCondition {
    Clear,
    PartlyCloudy,
    Cloudy,
    Fog,
    Drizzle,
    Rain,
    Snow,
    Thunderstorm,
    Unknown,
}

impl WeatherCondition {
    /// Convert WMO weather code to condition
    /// See: <https://open-meteo.com/en/docs#weathervariables>
    #[must_use]
    pub const fn from_wmo_code(code: u8) -> Self {
        match code {
            0 => Self::Clear,
            1 | 2 => Self::PartlyCloudy,
            3 => Self::Cloudy,
            45 | 48 => Self::Fog,
            51..=55 => Self::Drizzle,
            61..=67 | 80..=82 => Self::Rain,
            71..=77 | 85 | 86 => Self::Snow,
            95..=99 => Self::Thunderstorm,
            _ => Self::Unknown,
        }
    }

    #[must_use]
    pub const fn icon(&self) -> &'static str {
        match self {
            Self::Clear => "☀️",
            Self::PartlyCloudy => "⛅",
            Self::Cloudy => "☁️",
            Self::Fog => "🌫️",
            Self::Drizzle => "🌦️",
            Self::Rain => "🌧️",
            Self::Snow => "❄️",
            Self::Thunderstorm => "⛈️",
            Self::Unknown => "❓",
        }
    }
}

// ============================================================================
// Hofvarpnir Types
// ============================================================================

/// A video from Hofvarpnir's `/api/v1/downloads` endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HofvarpnirVideo {
    /// Video ID (ULID).
    pub id: String,
    /// Video title.
    pub title: String,
    /// Platform video ID (e.g., `YouTube` video ID).
    pub platform_video_id: String,
    /// Platform name (e.g., "youtube").
    pub platform: String,
    /// Effective name of the source this video came from: the user's custom
    /// name, falling back to the channel/playlist title. `None` when the API
    /// reports neither.
    pub source_name: Option<String>,
    /// Thumbnail URL provided by the API.
    pub thumbnail_url: Option<String>,
    /// When the video was downloaded.
    pub downloaded_at: Option<DateTime<Utc>>,
    /// When the video was originally published.
    pub published_at: Option<DateTime<Utc>>,
}

/// Aggregated Hofvarpnir status for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HofvarpnirStatus {
    /// Recent downloads.
    pub videos: Vec<HofvarpnirVideo>,
    /// Total number of completed downloads.
    pub total_downloads: u64,
    /// When this data was fetched.
    pub fetched_at: DateTime<Utc>,
}

// ============================================================================
// Health Check Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub id: Ulid,
    pub name: String,
    pub url: String,
    pub status: HealthStatus,
    pub response_time_ms: Option<u32>,
    pub last_checked: DateTime<Utc>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthOverview {
    pub checks: Vec<HealthCheck>,
    pub healthy_count: usize,
    pub unhealthy_count: usize,
    pub fetched_at: DateTime<Utc>,
}

// ============================================================================
// Dashboard Configuration (runtime)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardConfig {
    pub searxng_url: String,
    pub weather_location: String,
    pub refresh_interval_seconds: u32,
}

#[cfg(test)]
mod tests {
    use super::WeatherCondition;

    /// Every variant, so `icon()` coverage cannot silently drift.
    const ALL_CONDITIONS: [WeatherCondition; 9] = [
        WeatherCondition::Clear,
        WeatherCondition::PartlyCloudy,
        WeatherCondition::Cloudy,
        WeatherCondition::Fog,
        WeatherCondition::Drizzle,
        WeatherCondition::Rain,
        WeatherCondition::Snow,
        WeatherCondition::Thunderstorm,
        WeatherCondition::Unknown,
    ];

    #[test]
    fn from_wmo_code_maps_every_range_boundary() {
        // Each pair brackets a range edge: last code outside, first code inside,
        // last code inside, first code outside.
        let cases: [(u8, WeatherCondition); 34] = [
            (0, WeatherCondition::Clear),
            (1, WeatherCondition::PartlyCloudy),
            (2, WeatherCondition::PartlyCloudy),
            (3, WeatherCondition::Cloudy),
            (4, WeatherCondition::Unknown),
            (44, WeatherCondition::Unknown),
            (45, WeatherCondition::Fog),
            (48, WeatherCondition::Fog),
            (49, WeatherCondition::Unknown),
            (50, WeatherCondition::Unknown),
            (51, WeatherCondition::Drizzle),
            (55, WeatherCondition::Drizzle),
            (56, WeatherCondition::Unknown),
            (60, WeatherCondition::Unknown),
            (61, WeatherCondition::Rain),
            (67, WeatherCondition::Rain),
            (68, WeatherCondition::Unknown),
            (70, WeatherCondition::Unknown),
            (71, WeatherCondition::Snow),
            (77, WeatherCondition::Snow),
            (78, WeatherCondition::Unknown),
            (79, WeatherCondition::Unknown),
            (80, WeatherCondition::Rain),
            (82, WeatherCondition::Rain),
            (83, WeatherCondition::Unknown),
            (84, WeatherCondition::Unknown),
            (85, WeatherCondition::Snow),
            (86, WeatherCondition::Snow),
            (87, WeatherCondition::Unknown),
            (94, WeatherCondition::Unknown),
            (95, WeatherCondition::Thunderstorm),
            (99, WeatherCondition::Thunderstorm),
            (100, WeatherCondition::Unknown),
            (255, WeatherCondition::Unknown),
        ];

        for (code, expected) in cases {
            assert_eq!(
                WeatherCondition::from_wmo_code(code),
                expected,
                "WMO code {code}"
            );
        }
    }

    #[test]
    fn from_wmo_code_maps_thunderstorm_range() {
        for code in 95..=99 {
            assert_eq!(
                WeatherCondition::from_wmo_code(code),
                WeatherCondition::Thunderstorm,
                "WMO code {code}"
            );
        }
    }

    #[test]
    fn from_wmo_code_yields_cloudy_only_for_overcast() {
        // WMO 3 is "overcast" and is the sole source of `Cloudy`. It used to be
        // folded into the `1..=3 => PartlyCloudy` arm, which left `Cloudy` — and
        // its ☁️ icon — unreachable for every input.
        for code in 0..=u8::MAX {
            let is_cloudy = WeatherCondition::from_wmo_code(code) == WeatherCondition::Cloudy;
            assert_eq!(is_cloudy, code == 3, "WMO code {code}");
        }
    }

    #[test]
    fn from_wmo_code_drops_freezing_drizzle_to_unknown() {
        // BUG: WMO 56/57 are freezing drizzle but fall past the 51..=55 arm and
        // land in the catch-all, so they render as "❓" instead of drizzle.
        for code in [56, 57] {
            assert_eq!(
                WeatherCondition::from_wmo_code(code),
                WeatherCondition::Unknown,
                "WMO code {code}"
            );
        }
    }

    #[test]
    fn icon_is_non_empty_for_every_variant() {
        for condition in ALL_CONDITIONS {
            assert!(
                !condition.icon().is_empty(),
                "{condition:?} has an empty icon"
            );
        }
    }

    #[test]
    fn icon_is_distinct_for_every_variant() {
        let icons: Vec<&str> = ALL_CONDITIONS.iter().map(WeatherCondition::icon).collect();
        for (i, left) in icons.iter().enumerate() {
            for right in icons.iter().skip(i + 1) {
                assert_ne!(left, right, "duplicate icon {left}");
            }
        }
    }
}
