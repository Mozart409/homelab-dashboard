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
// Pinchflat Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinchflatVideo {
    pub id: Ulid,
    pub title: String,
    pub channel: String,
    pub thumbnail_url: Option<String>,
    pub duration_seconds: u32,
    pub downloaded_at: DateTime<Utc>,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinchflatStatus {
    pub videos: Vec<PinchflatVideo>,
    pub total_downloads: u64,
    pub is_downloading: bool,
    pub fetched_at: DateTime<Utc>,
}

// ============================================================================
// Proxmox Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxmoxNode {
    pub name: String,
    pub status: NodeStatus,
    pub cpu_usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
    pub uptime_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxmoxVm {
    pub vmid: u32,
    pub name: String,
    pub node: String,
    pub status: VmStatus,
    pub vm_type: VmType,
    pub cpu_usage: f64,
    pub memory_used: u64,
    pub memory_total: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NodeStatus {
    Online,
    Offline,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VmStatus {
    Running,
    Stopped,
    Paused,
    Suspended,
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum VmType {
    Qemu,
    Lxc,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxmoxStatus {
    pub nodes: Vec<ProxmoxNode>,
    pub vms: Vec<ProxmoxVm>,
    pub fetched_at: DateTime<Utc>,
}

// ============================================================================
// Jellyfin Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JellyfinStatus {
    pub server_name: String,
    pub version: String,
    pub active_streams: u32,
    pub total_movies: u32,
    pub total_series: u32,
    pub total_episodes: u32,
    pub recently_added: Vec<JellyfinItem>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JellyfinItem {
    pub id: String,
    pub name: String,
    pub item_type: JellyfinItemType,
    pub series_name: Option<String>,
    pub image_url: Option<String>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum JellyfinItemType {
    Movie,
    Series,
    Episode,
    Other,
}

// ============================================================================
// Home Assistant Types
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeAssistantStatus {
    pub version: String,
    pub entities: Vec<HomeAssistantEntity>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeAssistantEntity {
    pub entity_id: String,
    pub friendly_name: String,
    pub state: String,
    pub unit: Option<String>,
    pub icon: Option<String>,
    pub last_changed: DateTime<Utc>,
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
// SearXNG Types (for client-side search)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub content: Option<String>,
    pub engine: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub suggestions: Vec<String>,
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
