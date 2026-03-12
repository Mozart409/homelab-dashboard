//! Pinchflat video downloader API integration.
//!
//! Uses the official Pinchflat API endpoints:
//! - `/api/media/recent_downloads` for recent downloads
//! - `/api/stats` for statistics

use crate::types::{PinchflatStatus, PinchflatVideo};
use leptos::prelude::*;

// ============================================================================
// Server Functions
// ============================================================================

/// Fetch recent downloads from Pinchflat.
///
/// Uses the `/api/media/recent_downloads` and `/api/stats` endpoints.
#[server]
pub async fn get_pinchflat_status(limit: Option<u32>) -> Result<PinchflatStatus, ServerFnError> {
    use chrono::{DateTime, Utc};
    use moka::future::Cache;
    use serde::Deserialize;
    use std::sync::LazyLock;
    use std::time::Duration;

    // API response types matching Pinchflat's OpenAPI spec

    /// Nested source object in media item response.
    #[derive(Debug, Deserialize)]
    struct ApiSource {
        custom_name: String,
        #[allow(dead_code)]
        collection_name: String,
    }

    /// Media item from `/api/media/recent_downloads`.
    #[derive(Debug, Deserialize)]
    struct ApiMediaItem {
        id: i64,
        uuid: String,
        title: String,
        media_id: String,
        #[serde(default)]
        source: Option<ApiSource>,
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
        #[allow(dead_code)]
        source_count: u64,
        #[allow(dead_code)]
        media_profile_count: u64,
        #[allow(dead_code)]
        total_download_size_bytes: u64,
    }

    // Shared cache across requests
    static PINCHFLAT_CACHE: LazyLock<Cache<u32, PinchflatStatus>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(60))
            .max_capacity(5)
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
            channel: item
                .source
                .map_or_else(|| "Unknown".to_string(), |s| s.custom_name),
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
