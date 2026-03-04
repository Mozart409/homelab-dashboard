//! Jellyfin media server API integration.

use crate::types::{JellyfinItem, JellyfinItemType, JellyfinStatus};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::Deserialize;

// ============================================================================
// Jellyfin API Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SystemInfo {
    server_name: String,
    version: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ItemsResponse {
    items: Vec<ApiItem>,
    #[allow(dead_code)]
    total_record_count: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ApiItem {
    id: String,
    name: String,
    #[serde(rename = "Type")]
    item_type: String,
    series_name: Option<String>,
    image_tags: Option<ImageTags>,
    date_created: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ImageTags {
    primary: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SessionInfo {
    // We only care about counting active sessions
}

// ============================================================================
// Server Functions
// ============================================================================

/// Fetch Jellyfin server status and recently added items.
#[server]
pub async fn get_jellyfin_status() -> Result<JellyfinStatus, ServerFnError> {
    use std::sync::LazyLock;
    use std::time::Duration;
    use moka::future::Cache;

    static JELLYFIN_CACHE: LazyLock<Cache<(), JellyfinStatus>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(60))
            .max_capacity(1)
            .build()
    });

    if let Some(cached) = JELLYFIN_CACHE.get(&()).await {
        tracing::debug!("Jellyfin cache hit");
        return Ok(cached);
    }

    let base_url = std::env::var("JELLYFIN_URL")
        .map_err(|_| ServerFnError::new("JELLYFIN_URL not configured"))?;
    let api_key = std::env::var("JELLYFIN_API_KEY")
        .map_err(|_| ServerFnError::new("JELLYFIN_API_KEY not configured"))?;

    let client = reqwest::Client::new();

    // Fetch system info
    let system_info: SystemInfo = client
        .get(format!("{base_url}/System/Info"))
        .header("X-Emby-Token", &api_key)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch Jellyfin system info: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse Jellyfin system info: {e}")))?;

    // Fetch active sessions count
    let sessions_response = client
        .get(format!("{base_url}/Sessions"))
        .header("X-Emby-Token", &api_key)
        .send()
        .await;
    let sessions: Vec<SessionInfo> = match sessions_response {
        Ok(r) => r.json().await.unwrap_or_default(),
        Err(_) => vec![],
    };

    // Fetch library stats
    let movies_count = fetch_item_count(&client, &base_url, &api_key, "Movie").await;
    let series_count = fetch_item_count(&client, &base_url, &api_key, "Series").await;
    let episodes_count = fetch_item_count(&client, &base_url, &api_key, "Episode").await;

    // Fetch recently added
    let recently_added_response = client
        .get(format!("{base_url}/Items/Latest"))
        .header("X-Emby-Token", &api_key)
        .query(&[
            ("Limit", "10"),
            ("IncludeItemTypes", "Movie,Episode"),
            ("Fields", "DateCreated,SeriesName"),
        ])
        .send()
        .await;
    let recently_added: ItemsResponse = match recently_added_response {
        Ok(r) => r.json().await.unwrap_or(ItemsResponse {
            items: vec![],
            total_record_count: 0,
        }),
        Err(_) => ItemsResponse {
            items: vec![],
            total_record_count: 0,
        },
    };

    let recently_added: Vec<JellyfinItem> = recently_added
        .items
        .into_iter()
        .map(|item| {
            let image_url = item.image_tags.as_ref().and_then(|tags| {
                tags.primary.as_ref().map(|_| {
                    format!("{}/Items/{}/Images/Primary?maxHeight=200", base_url, item.id)
                })
            });

            JellyfinItem {
                id: item.id,
                name: item.name,
                item_type: match item.item_type.as_str() {
                    "Movie" => JellyfinItemType::Movie,
                    "Series" => JellyfinItemType::Series,
                    "Episode" => JellyfinItemType::Episode,
                    _ => JellyfinItemType::Other,
                },
                series_name: item.series_name,
                image_url,
                added_at: item.date_created.unwrap_or_else(Utc::now),
            }
        })
        .collect();

    let status = JellyfinStatus {
        server_name: system_info.server_name,
        version: system_info.version,
        #[allow(clippy::cast_possible_truncation)]
        active_streams: sessions.len() as u32,
        total_movies: movies_count,
        total_series: series_count,
        total_episodes: episodes_count,
        recently_added,
        fetched_at: Utc::now(),
    };

    JELLYFIN_CACHE.insert((), status.clone()).await;
    Ok(status)
}

async fn fetch_item_count(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    item_type: &str,
) -> u32 {
    #[derive(Deserialize)]
    #[serde(rename_all = "PascalCase")]
    struct CountResponse {
        total_record_count: u32,
    }

    let response = client
        .get(format!("{base_url}/Items"))
        .header("X-Emby-Token", api_key)
        .query(&[
            ("IncludeItemTypes", item_type),
            ("Recursive", "true"),
            ("Limit", "0"),
        ])
        .send()
        .await;

    match response {
        Ok(r) => r.json::<CountResponse>().await
            .map(|c| c.total_record_count)
            .unwrap_or(0),
        Err(_) => 0,
    }
}
