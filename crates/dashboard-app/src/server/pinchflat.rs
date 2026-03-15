//! Pinchflat video downloader API integration.
//!
//! Uses the official Pinchflat API endpoints:
//! - `/api/media/recent_downloads` for recent downloads
//! - `/api/stats` for statistics
//! - `/sources` for source/channel names

use crate::types::{PinchflatStatus, PinchflatVideo};
use leptos::prelude::*;

// ============================================================================
// Server Functions
// ============================================================================

/// Fetch recent downloads from Pinchflat.
///
/// Uses the `/api/media/recent_downloads`, `/api/stats`, and `/sources` endpoints.
#[server]
#[allow(clippy::too_many_lines)]
pub async fn get_pinchflat_status(limit: Option<u32>) -> Result<PinchflatStatus, ServerFnError> {
    use chrono::{DateTime, Utc};
    use moka::future::Cache;
    use serde::Deserialize;
    use std::collections::HashMap;
    use std::sync::LazyLock;
    use std::time::Duration;

    // API response types matching Pinchflat's OpenAPI spec

    /// Source object from `/sources` endpoint.
    #[derive(Debug, Deserialize)]
    struct ApiSource {
        id: i64,
        custom_name: String,
    }

    /// Response from `/sources`.
    #[derive(Debug, Deserialize)]
    struct SourcesResponse {
        data: Vec<ApiSource>,
    }

    /// Media item from `/api/media/recent_downloads`.
    #[derive(Debug, Deserialize)]
    struct ApiMediaItem {
        id: i64,
        uuid: String,
        title: String,
        media_id: String,
        source_id: i64,
        media_downloaded_at: Option<DateTime<Utc>>,
        uploaded_at: Option<DateTime<Utc>>,
    }

    /// Response from `/api/media/recent_downloads`.
    #[derive(Debug, Deserialize)]
    struct RecentDownloadsResponse {
        data: Vec<ApiMediaItem>,
    }

    /// Response from `/api/stats`.
    #[derive(Debug, Deserialize)]
    struct StatsResponse {
        media_item_count: u64,
    }

    // Shared cache across requests
    static PINCHFLAT_CACHE: LazyLock<Cache<u32, PinchflatStatus>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(60))
            .max_capacity(5)
            .build()
    });

    // Cache for source names (longer TTL since they rarely change)
    static SOURCES_CACHE: LazyLock<Cache<(), HashMap<i64, String>>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(300))
            .build()
    });

    let limit = limit.unwrap_or(3);

    if let Some(cached) = PINCHFLAT_CACHE.get(&limit).await {
        tracing::debug!("Pinchflat cache hit");
        return Ok(cached);
    }

    let base_url = std::env::var("PINCHFLAT_URL")
        .map_err(|_| ServerFnError::new("PINCHFLAT_URL not configured"))?;

    // Optional API key for authentication
    let api_key = std::env::var("PINCHFLAT_API_KEY").ok();

    let client = reqwest::Client::new();

    // Build request with optional auth header
    let build_request = |url: String| {
        let mut req = client.get(url);
        if let Some(ref key) = api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        req
    };

    // Fetch or use cached source names
    let source_names: HashMap<i64, String> = if let Some(cached) = SOURCES_CACHE.get(&()).await {
        cached
    } else {
        let sources_url = format!("{base_url}/sources");
        let sources_response: SourcesResponse = build_request(sources_url)
            .send()
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to fetch sources: {e}")))?
            .json()
            .await
            .map_err(|e| ServerFnError::new(format!("Failed to parse sources: {e}")))?;

        let map: HashMap<i64, String> = sources_response
            .data
            .into_iter()
            .map(|s| (s.id, s.custom_name))
            .collect();

        SOURCES_CACHE.insert((), map.clone()).await;
        map
    };

    // Fetch recent downloads
    let recent_url = format!("{base_url}/api/media/recent_downloads?limit={limit}");
    let recent_response: RecentDownloadsResponse = build_request(recent_url)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch recent downloads: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse recent downloads: {e}")))?;

    // Fetch stats for total count
    let stats_url = format!("{base_url}/api/stats");
    let stats_response: StatsResponse = build_request(stats_url)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch stats: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse stats: {e}")))?;

    let videos: Vec<PinchflatVideo> = recent_response
        .data
        .into_iter()
        .map(|item| PinchflatVideo {
            id: item.id,
            uuid: item.uuid,
            title: item.title,
            media_id: item.media_id,
            channel: source_names
                .get(&item.source_id)
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            downloaded_at: item.media_downloaded_at,
            uploaded_at: item.uploaded_at,
        })
        .collect();

    let status = PinchflatStatus {
        videos,
        total_downloads: stats_response.media_item_count,
        fetched_at: Utc::now(),
    };

    PINCHFLAT_CACHE.insert(limit, status.clone()).await;
    Ok(status)
}
