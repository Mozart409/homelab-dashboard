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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            1..=3 => Self::PartlyCloudy,
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
