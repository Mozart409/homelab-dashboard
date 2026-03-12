//! Pinchflat video downloader API integration.
//!
//! Note: This assumes you'll implement a REST API in your Pinchflat fork.
//! Adjust the response types to match your actual API.

use crate::types::{PinchflatStatus, PinchflatVideo};
use leptos::prelude::*;

// ============================================================================
// Server Functions
// ============================================================================

/// Fetch recent downloads from Pinchflat.
#[server]
pub async fn get_pinchflat_status(limit: Option<usize>) -> Result<PinchflatStatus, ServerFnError> {
    use chrono::{DateTime, Utc};
    use moka::future::Cache;
    use serde::Deserialize;
    use std::sync::LazyLock;
    use std::time::Duration;
    use ulid::Ulid;

    #[derive(Debug, Deserialize)]
    struct ApiVideo {
        #[allow(dead_code)]
        id: String,
        title: String,
        channel: String,
        thumbnail_url: Option<String>,
        duration_seconds: u32,
        downloaded_at: DateTime<Utc>,
        file_path: String,
    }

    #[derive(Debug, Deserialize)]
    struct ApiStatus {
        videos: Vec<ApiVideo>,
        total_downloads: u64,
        is_downloading: bool,
    }

    // Shared cache across requests
    static PINCHFLAT_CACHE: LazyLock<Cache<usize, PinchflatStatus>> = LazyLock::new(|| {
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

    // Optional API key if your fork requires auth
    let api_key = std::env::var("PINCHFLAT_API_KEY").ok();

    let client = reqwest::Client::new();
    let mut request = client.get(format!("{base_url}/api/videos/recent?limit={limit}"));

    if let Some(key) = api_key {
        request = request.header("Authorization", format!("Bearer {key}"));
    }

    let response: ApiStatus = request
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch Pinchflat status: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse Pinchflat response: {e}")))?;

    let videos: Vec<PinchflatVideo> = response
        .videos
        .into_iter()
        .map(|v| PinchflatVideo {
            // Generate a ULID from the existing ID or create new one
            id: Ulid::new(),
            title: v.title,
            channel: v.channel,
            thumbnail_url: v.thumbnail_url,
            duration_seconds: v.duration_seconds,
            downloaded_at: v.downloaded_at,
            file_path: v.file_path,
        })
        .collect();

    let status = PinchflatStatus {
        videos,
        total_downloads: response.total_downloads,
        is_downloading: response.is_downloading,
        fetched_at: Utc::now(),
    };

    PINCHFLAT_CACHE.insert(limit, status.clone()).await;
    Ok(status)
}
