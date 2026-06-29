//! Hofvarpnir video archival system API integration.
//!
//! Uses the Hofvarpnir API endpoints:
//! - `/api/v1/downloads` for download listing
//! - `/api/v1/system/status` for statistics

use crate::types::{HofvarpnirStatus, HofvarpnirVideo};
use color_eyre::eyre::{Result, WrapErr};

/// Fetch recent downloads from Hofvarpnir.
///
/// Uses the `/api/v1/downloads` and `/api/v1/system/status` endpoints.
/// Cached for 60 seconds.
#[allow(clippy::too_many_lines)]
pub async fn get_hofvarpnir_status(limit: Option<u32>) -> Result<HofvarpnirStatus> {
    use chrono::{DateTime, Utc};
    use moka::future::Cache;
    use serde::Deserialize;
    use std::sync::LazyLock;
    use std::time::Duration;

    // API response types matching Hofvarpnir's OpenAPI spec

    /// Video from `/api/v1/downloads`.
    #[derive(Debug, Deserialize)]
    struct ApiVideo {
        id: String,
        platform: String,
        platform_video_id: String,
        title: String,
        downloaded_at: Option<DateTime<Utc>>,
        published_at: Option<DateTime<Utc>>,
        thumbnail_url: Option<String>,
    }

    /// Response from `/api/v1/system/status`.
    #[derive(Debug, Deserialize)]
    struct SystemStatusResponse {
        statistics: StatisticsResponse,
    }

    #[derive(Debug, Deserialize)]
    struct StatisticsResponse {
        completed: i64,
    }

    // Shared cache across requests
    static HOFVARPNIR_CACHE: LazyLock<Cache<u32, HofvarpnirStatus>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_mins(1))
            .max_capacity(5)
            .build()
    });

    let limit = limit.unwrap_or(3);

    if let Some(cached) = HOFVARPNIR_CACHE.get(&limit).await {
        tracing::debug!("Hofvarpnir cache hit");
        return Ok(cached);
    }

    let base_url = std::env::var("HOFVARPNIR_URL").wrap_err("HOFVARPNIR_URL not configured")?;

    // Optional API key for authentication
    let api_key = std::env::var("HOFVARPNIR_API_KEY").ok();

    let client = reqwest::Client::new();

    // Build request with optional auth header
    let build_request = |url: String| {
        let mut req = client.get(url);
        if let Some(ref key) = api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        req
    };

    // Fetch completed downloads
    let downloads_url = format!("{base_url}/api/v1/downloads?status=Completed");
    let mut downloads: Vec<ApiVideo> = build_request(downloads_url)
        .send()
        .await
        .wrap_err("Failed to fetch downloads")?
        .json()
        .await
        .wrap_err("Failed to parse downloads")?;

    // Sort by downloaded_at descending, take the most recent N
    downloads.sort_by_key(|d| std::cmp::Reverse(d.downloaded_at));
    downloads.truncate(limit as usize);

    // Fetch system status for total count
    let status_url = format!("{base_url}/api/v1/system/status");
    let system_status: SystemStatusResponse = build_request(status_url)
        .send()
        .await
        .wrap_err("Failed to fetch system status")?
        .json()
        .await
        .wrap_err("Failed to parse system status")?;

    let videos: Vec<HofvarpnirVideo> = downloads
        .into_iter()
        .map(|item| HofvarpnirVideo {
            id: item.id,
            title: item.title,
            platform_video_id: item.platform_video_id,
            platform: item.platform,
            thumbnail_url: item.thumbnail_url,
            downloaded_at: item.downloaded_at,
            published_at: item.published_at,
        })
        .collect();

    let status = HofvarpnirStatus {
        videos,
        total_downloads: system_status.statistics.completed.cast_unsigned(),
        fetched_at: Utc::now(),
    };

    HOFVARPNIR_CACHE.insert(limit, status.clone()).await;
    Ok(status)
}
