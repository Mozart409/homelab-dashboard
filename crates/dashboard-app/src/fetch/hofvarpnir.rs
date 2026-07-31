//! Hofvarpnir video archival system API integration.
//!
//! Uses the Hofvarpnir API endpoints:
//! - `/api/v1/downloads` for download listing
//! - `/api/v1/system/status` for statistics
//!
//! [`get_hofvarpnir_status`] is the public entry point: it reads the
//! `HOFVARPNIR_URL` / `HOFVARPNIR_API_KEY` environment variables and owns the
//! response cache. The HTTP work lives in [`fetch_status`], which takes the
//! base URL and API key explicitly and neither reads the environment nor
//! touches the cache, so tests can point it at a mock server.

use crate::types::{HofvarpnirStatus, HofvarpnirVideo};
use chrono::{DateTime, Utc};
use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;

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
    /// Source's effective display name: `custom_name`, falling back to
    /// `channel_title`, then the source URL. Matches Hofvarpnir's web UI.
    #[serde(default)]
    source_display_name: Option<String>,
    /// User-provided custom name for the source, if set.
    #[serde(default)]
    source_custom_name: Option<String>,
    /// Channel/playlist title as reported by the platform.
    #[serde(default)]
    source_channel_title: Option<String>,
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

/// Resolve the effective source name for a video.
///
/// Prefers the custom name, then the channel/playlist title. The API's own
/// `source_display_name` degrades to the raw source URL when neither is set,
/// so it is only accepted when it isn't one.
fn source_name(
    custom_name: Option<String>,
    channel_title: Option<String>,
    display_name: Option<String>,
) -> Option<String> {
    custom_name
        .or(channel_title)
        .or_else(|| {
            display_name.filter(|n| !n.starts_with("http://") && !n.starts_with("https://"))
        })
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty())
}

/// Map an API video onto the dashboard's [`HofvarpnirVideo`].
fn map_video(item: ApiVideo) -> HofvarpnirVideo {
    HofvarpnirVideo {
        id: item.id,
        title: item.title,
        platform_video_id: item.platform_video_id,
        platform: item.platform,
        source_name: source_name(
            item.source_custom_name,
            item.source_channel_title,
            item.source_display_name,
        ),
        thumbnail_url: item.thumbnail_url,
        downloaded_at: item.downloaded_at,
        published_at: item.published_at,
    }
}

/// Sort by `downloaded_at` descending and keep the most recent `limit` videos.
fn sort_and_truncate(videos: &mut Vec<ApiVideo>, limit: u32) {
    videos.sort_by_key(|d| std::cmp::Reverse(d.downloaded_at));
    videos.truncate(limit as usize);
}

/// Fetch recent downloads and the completed count from a Hofvarpnir host.
///
/// No environment access and no caching — the caller supplies everything.
pub(crate) async fn fetch_status(
    base_url: &str,
    api_key: Option<&str>,
    limit: u32,
) -> Result<HofvarpnirStatus> {
    // Bounded like the health checker's client: an unbounded request would stall
    // the SSE render loop for as long as the OS keeps the socket open.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .wrap_err("Failed to create HTTP client")?;

    // Build request with optional auth header
    let build_request = |url: String| {
        let mut req = client.get(url);
        if let Some(key) = api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }
        req
    };

    // Fetch completed downloads
    // Normalised like `SearchEngine::action_url`, so a configured
    // `https://host/` does not produce a double slash in the request path.
    let base_url = base_url.trim_end_matches('/');

    let downloads_url = format!("{base_url}/api/v1/downloads?status=Completed");
    let mut downloads: Vec<ApiVideo> = build_request(downloads_url)
        .send()
        .await
        .wrap_err("Failed to fetch downloads")?
        .json()
        .await
        .wrap_err("Failed to parse downloads")?;

    // Sort by downloaded_at descending, take the most recent N
    sort_and_truncate(&mut downloads, limit);

    // Fetch system status for total count
    let status_url = format!("{base_url}/api/v1/system/status");
    let system_status: SystemStatusResponse = build_request(status_url)
        .send()
        .await
        .wrap_err("Failed to fetch system status")?
        .json()
        .await
        .wrap_err("Failed to parse system status")?;

    let videos: Vec<HofvarpnirVideo> = downloads.into_iter().map(map_video).collect();

    Ok(HofvarpnirStatus {
        videos,
        total_downloads: system_status.statistics.completed.cast_unsigned(),
        fetched_at: Utc::now(),
    })
}

/// Fetch recent downloads from Hofvarpnir.
///
/// Uses the `/api/v1/downloads` and `/api/v1/system/status` endpoints.
/// Cached for 60 seconds.
pub async fn get_hofvarpnir_status(limit: Option<u32>) -> Result<HofvarpnirStatus> {
    use moka::future::Cache;
    use std::sync::LazyLock;
    use std::time::Duration;

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

    let status = fetch_status(&base_url, api_key.as_deref(), limit).await?;

    HOFVARPNIR_CACHE.insert(limit, status.clone()).await;
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::{
        ApiVideo, SystemStatusResponse, fetch_status, map_video, sort_and_truncate, source_name,
    };
    use chrono::{DateTime, TimeZone, Utc};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn at(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 1, day, 12, 0, 0)
            .single()
            .expect("valid timestamp")
    }

    fn video(id: &str, downloaded_at: Option<DateTime<Utc>>) -> ApiVideo {
        ApiVideo {
            id: id.to_string(),
            platform: "youtube".to_string(),
            platform_video_id: "vid".to_string(),
            title: "A title".to_string(),
            downloaded_at,
            published_at: None,
            thumbnail_url: None,
            source_display_name: None,
            source_custom_name: None,
            source_channel_title: None,
        }
    }

    fn ids(videos: &[ApiVideo]) -> Vec<&str> {
        videos.iter().map(|v| v.id.as_str()).collect()
    }

    /// Shorthand for the `Option<String>` values these helpers deal in.
    #[allow(clippy::unnecessary_wraps)] // the `Some` is the point
    fn some(s: &str) -> Option<String> {
        Some(s.to_string())
    }

    // ------------------------------------------------------------------
    // source_name
    // ------------------------------------------------------------------

    #[test]
    fn source_name_prefers_the_custom_name() {
        assert_eq!(
            source_name(some("Custom"), some("Channel"), some("Display")),
            some("Custom")
        );
    }

    #[test]
    fn source_name_uses_the_custom_name_alone() {
        assert_eq!(source_name(some("Custom"), None, None), some("Custom"));
    }

    #[test]
    fn source_name_falls_back_to_the_channel_title() {
        assert_eq!(source_name(None, some("Channel"), None), some("Channel"));
    }

    #[test]
    fn source_name_rejects_a_url_shaped_display_name() {
        assert_eq!(
            source_name(None, None, some("https://youtube.com/@someone")),
            None
        );
        assert_eq!(
            source_name(None, None, some("http://youtube.com/@someone")),
            None
        );
    }

    #[test]
    fn source_name_accepts_a_name_shaped_display_name() {
        assert_eq!(
            source_name(None, None, some("Some Channel")),
            some("Some Channel")
        );
    }

    #[test]
    fn source_name_trims_whitespace() {
        assert_eq!(source_name(some("  Custom  "), None, None), some("Custom"));
    }

    #[test]
    fn source_name_whitespace_only_custom_shadows_the_channel_title() {
        // BUG: `.or` treats `Some("   ")` as present, so the channel title is
        // never consulted; the blank name then trims to "" and is filtered
        // away, leaving no source name at all.
        assert_eq!(source_name(some("   "), some("Channel"), None), None);
    }

    #[test]
    fn source_name_all_none_is_none() {
        assert_eq!(source_name(None, None, None), None);
    }

    // ------------------------------------------------------------------
    // map_video
    // ------------------------------------------------------------------

    #[test]
    fn map_video_carries_every_field_across() {
        let mut item = video("01", Some(at(2)));
        item.published_at = Some(at(1));
        item.thumbnail_url = some("https://img/1.jpg");
        item.source_channel_title = some("Channel");

        let mapped = map_video(item);

        assert_eq!(mapped.id, "01");
        assert_eq!(mapped.title, "A title");
        assert_eq!(mapped.platform, "youtube");
        assert_eq!(mapped.platform_video_id, "vid");
        assert_eq!(mapped.source_name, some("Channel"));
        assert_eq!(mapped.thumbnail_url, some("https://img/1.jpg"));
        assert_eq!(mapped.downloaded_at, Some(at(2)));
        assert_eq!(mapped.published_at, Some(at(1)));
    }

    // ------------------------------------------------------------------
    // sort_and_truncate
    // ------------------------------------------------------------------

    #[test]
    fn sort_and_truncate_orders_newest_first() {
        let mut videos = vec![
            video("old", Some(at(1))),
            video("new", Some(at(3))),
            video("mid", Some(at(2))),
        ];
        sort_and_truncate(&mut videos, 10);

        assert_eq!(ids(&videos), ["new", "mid", "old"]);
    }

    #[test]
    fn sort_and_truncate_sorts_missing_timestamps_last() {
        // `Reverse(None)` is greater than `Reverse(Some(_))`, so undated
        // downloads sink to the bottom.
        let mut videos = vec![
            video("undated", None),
            video("old", Some(at(1))),
            video("new", Some(at(2))),
        ];
        sort_and_truncate(&mut videos, 10);

        assert_eq!(ids(&videos), ["new", "old", "undated"]);
    }

    #[test]
    fn sort_and_truncate_keeps_input_order_when_all_undated() {
        // `sort_by_key` is stable.
        let mut videos = vec![video("a", None), video("b", None), video("c", None)];
        sort_and_truncate(&mut videos, 10);

        assert_eq!(ids(&videos), ["a", "b", "c"]);
    }

    #[test]
    fn sort_and_truncate_keeps_input_order_on_ties() {
        let mut videos = vec![
            video("a", Some(at(1))),
            video("b", Some(at(1))),
            video("c", Some(at(2))),
        ];
        sort_and_truncate(&mut videos, 10);

        assert_eq!(ids(&videos), ["c", "a", "b"]);
    }

    #[test]
    fn sort_and_truncate_with_limit_zero_drops_everything() {
        let mut videos = vec![video("a", Some(at(1)))];
        sort_and_truncate(&mut videos, 0);

        assert!(videos.is_empty());
    }

    #[test]
    fn sort_and_truncate_with_a_limit_beyond_the_length_keeps_everything() {
        let mut videos = vec![video("a", Some(at(1))), video("b", Some(at(2)))];
        sort_and_truncate(&mut videos, 100);

        assert_eq!(ids(&videos), ["b", "a"]);
    }

    #[test]
    fn sort_and_truncate_applies_the_limit_after_sorting() {
        let mut videos = vec![
            video("old", Some(at(1))),
            video("new", Some(at(3))),
            video("mid", Some(at(2))),
        ];
        sort_and_truncate(&mut videos, 2);

        assert_eq!(ids(&videos), ["new", "mid"]);
    }

    // ------------------------------------------------------------------
    // total_downloads
    // ------------------------------------------------------------------

    #[test]
    fn negative_completed_count_wraps_around() {
        // BUG: `cast_unsigned` reinterprets the sign bit, so a negative count
        // from the API renders as an absurd total instead of an error.
        let response: SystemStatusResponse =
            serde_json::from_str(r#"{"statistics":{"completed":-1}}"#).expect("valid JSON");

        assert_eq!(
            response.statistics.completed.cast_unsigned(),
            18_446_744_073_709_551_615_u64
        );
    }

    // ------------------------------------------------------------------
    // fetch_status (mock HTTP)
    // ------------------------------------------------------------------

    const DOWNLOADS_BODY: &str = r#"[
        {
            "id": "01",
            "platform": "youtube",
            "platform_video_id": "aaa",
            "title": "Older",
            "downloaded_at": "2026-01-01T12:00:00Z",
            "published_at": null,
            "thumbnail_url": null,
            "source_channel_title": "Channel A"
        },
        {
            "id": "02",
            "platform": "youtube",
            "platform_video_id": "bbb",
            "title": "Newer",
            "downloaded_at": "2026-01-03T12:00:00Z",
            "published_at": "2026-01-02T12:00:00Z",
            "thumbnail_url": "https://img/2.jpg",
            "source_display_name": "https://youtube.com/@b"
        }
    ]"#;

    const STATUS_BODY: &str = r#"{"statistics":{"completed":42}}"#;

    async fn mock_both_endpoints(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/api/v1/downloads"))
            .and(query_param("status", "Completed"))
            .respond_with(ResponseTemplate::new(200).set_body_string(DOWNLOADS_BODY))
            .mount(server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/system/status"))
            .respond_with(ResponseTemplate::new(200).set_body_string(STATUS_BODY))
            .mount(server)
            .await;
    }

    #[tokio::test]
    async fn fetch_status_happy_path() {
        let server = MockServer::start().await;
        mock_both_endpoints(&server).await;

        let status = fetch_status(&server.uri(), None, 3)
            .await
            .expect("both endpoints respond");

        assert_eq!(status.total_downloads, 42);
        assert_eq!(status.videos.len(), 2);
        // Newest first.
        assert_eq!(status.videos[0].id, "02");
        // A URL-shaped display name yields no source name.
        assert_eq!(status.videos[0].source_name, None);
        assert_eq!(status.videos[1].id, "01");
        assert_eq!(status.videos[1].source_name, some("Channel A"));
    }

    #[tokio::test]
    async fn fetch_status_applies_the_limit() {
        let server = MockServer::start().await;
        mock_both_endpoints(&server).await;

        let status = fetch_status(&server.uri(), None, 1)
            .await
            .expect("both endpoints respond");

        assert_eq!(status.videos.len(), 1);
        assert_eq!(status.videos[0].id, "02");
    }

    #[tokio::test]
    async fn fetch_status_sends_the_bearer_token_when_a_key_is_configured() {
        let server = MockServer::start().await;
        mock_both_endpoints(&server).await;

        fetch_status(&server.uri(), Some("k"), 3)
            .await
            .expect("both endpoints respond");

        let requests = server
            .received_requests()
            .await
            .expect("request recording is enabled");
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert_eq!(
                request
                    .headers
                    .get("authorization")
                    .map(|v| v.to_str().expect("ascii header")),
                Some("Bearer k"),
                "missing auth header on {}",
                request.url
            );
        }
    }

    #[tokio::test]
    async fn fetch_status_omits_the_auth_header_without_a_key() {
        let server = MockServer::start().await;
        mock_both_endpoints(&server).await;

        fetch_status(&server.uri(), None, 3)
            .await
            .expect("both endpoints respond");

        let requests = server
            .received_requests()
            .await
            .expect("request recording is enabled");
        assert_eq!(requests.len(), 2);
        for request in &requests {
            assert!(
                request.headers.get("authorization").is_none(),
                "unexpected auth header on {}",
                request.url
            );
        }
    }

    #[tokio::test]
    async fn fetch_status_hits_both_endpoints() {
        let server = MockServer::start().await;
        mock_both_endpoints(&server).await;

        fetch_status(&server.uri(), None, 3)
            .await
            .expect("both endpoints respond");

        let requests = server
            .received_requests()
            .await
            .expect("request recording is enabled");
        assert_eq!(requests[0].url.path(), "/api/v1/downloads");
        assert_eq!(requests[0].url.query(), Some("status=Completed"));
        assert_eq!(requests[1].url.path(), "/api/v1/system/status");
        assert_eq!(requests[1].url.query(), None);
    }

    #[tokio::test]
    async fn fetch_status_normalizes_a_trailing_slash_base_url() {
        // A configured `https://host/` used to produce `//api/v1/...`, unlike
        // `SearchEngine::action_url`, which has always trimmed trailing slashes.
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/downloads"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/system/status"))
            .respond_with(ResponseTemplate::new(200).set_body_string(STATUS_BODY))
            .mount(&server)
            .await;

        let status = fetch_status(&format!("{}///", server.uri()), None, 3)
            .await
            .expect("trailing slashes are trimmed before the path is appended");

        assert_eq!(status.total_downloads, 42);

        let requests = server
            .received_requests()
            .await
            .expect("request recording is enabled");
        assert_eq!(requests[0].url.path(), "/api/v1/downloads");
    }

    #[tokio::test]
    async fn fetch_status_downloads_transport_failure() {
        // Nothing is listening, so `send()` itself fails.
        let err = fetch_status("http://127.0.0.1:1", None, 3)
            .await
            .expect_err("connection refused");

        assert!(
            format!("{err:?}").contains("Failed to fetch downloads"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_status_downloads_500_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/downloads"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = fetch_status(&server.uri(), None, 3)
            .await
            .expect_err("a 500 must not yield a status");

        // NOTE: `reqwest` does not treat 5xx as a transport error, so the
        // failure surfaces at the decode step. "Failed to fetch downloads" is
        // reserved for connect/transport failures.
        assert!(
            format!("{err:?}").contains("Failed to parse downloads"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_status_system_status_500_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/downloads"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/system/status"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = fetch_status(&server.uri(), None, 3)
            .await
            .expect_err("a 500 must not yield a status");

        assert!(
            format!("{err:?}").contains("Failed to parse system status"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_status_system_status_transport_failure() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/downloads"))
            .respond_with(ResponseTemplate::new(200).set_body_string("[]"))
            .mount(&server)
            .await;
        // `/api/v1/system/status` is deliberately unmounted: wiremock answers
        // an unmatched request with 404, which decodes as a parse failure.
        let err = fetch_status(&server.uri(), None, 3)
            .await
            .expect_err("no status endpoint");

        assert!(
            format!("{err:?}").contains("Failed to parse system status"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_status_malformed_downloads_body() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v1/downloads"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{ not json"))
            .mount(&server)
            .await;

        let err = fetch_status(&server.uri(), None, 3)
            .await
            .expect_err("malformed body");

        assert!(
            format!("{err:?}").contains("Failed to parse downloads"),
            "unexpected error: {err:?}"
        );
    }
}
