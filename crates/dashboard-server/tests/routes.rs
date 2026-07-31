#![allow(clippy::multiple_crate_versions)]
//! HTTP-level tests for the dashboard router.
//!
//! Everything here drives the [`Router`] returned by
//! [`dashboard_server::build_router`] through `tower::ServiceExt::oneshot` — no
//! socket is bound and no outbound request is made. Cards that would need the
//! network are asserted on their inline-error path instead, which is the
//! contract those handlers are supposed to uphold (a failed fetch renders an
//! error card, it never fails the request).

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use http_body_util::BodyExt;
use tower::ServiceExt;

use dashboard_app::{DashboardConfig, Icon, QuickLink, SearchEngine, SearchEngineKind};
use dashboard_server::{AppState, build_router};

// ============================================================================
// Helpers
// ============================================================================

/// A config with a couple of quick links and a search engine, so the fully
/// offline parts of the page have something to render.
fn test_config() -> DashboardConfig {
    DashboardConfig {
        latitude: 48.14,
        longitude: 11.58,
        location_name: "Munich".to_string(),
        search: Some(SearchEngine {
            kind: SearchEngineKind::Searxng,
            url: "https://search.example.com".to_string(),
        }),
        quick_links: vec![
            QuickLink {
                name: "Grafana".to_string(),
                url: "http://grafana.example".to_string(),
                icon: Some(Icon::Text("📊".to_string())),
            },
            QuickLink {
                name: "Router".to_string(),
                url: "http://192.168.1.1".to_string(),
                icon: None,
            },
        ],
        health_check_names: vec!["Grafana".to_string(), "Prometheus".to_string()],
    }
}

/// A router whose static fallback points at a directory that does not exist,
/// for tests that only care about the routed endpoints.
fn app() -> Router {
    build_router(
        AppState {
            config: test_config(),
        },
        "tests/fixtures/no-such-static-dir",
    )
}

async fn get(app: Router, uri: &str) -> Response {
    app.oneshot(
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .expect("valid request"),
    )
    .await
    .expect("router is infallible")
}

async fn body_string(response: Response) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body collects")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("body is utf-8")
}

fn header_str(response: &Response, name: header::HeaderName) -> &str {
    response
        .headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
}

/// Create a unique directory under the system temp dir. Cleaned up by the
/// caller; there is no `tempfile` dependency in this workspace.
fn scratch_dir(tag: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "dashboard-server-test-{tag}-{}-{nanos}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

// ============================================================================
// GET /
// ============================================================================

#[tokio::test]
async fn index_renders_the_page_shell() {
    let response = get(app(), "/").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        header_str(&response, header::CONTENT_TYPE).starts_with("text/html"),
        "content-type was {:?}",
        header_str(&response, header::CONTENT_TYPE)
    );

    let body = body_string(response).await;

    // All four card shells are present, keyed by the ids the SSE stream pushes.
    for id in dashboard_server::CARD_IDS {
        assert!(
            body.contains(&format!("id=\"card-{id}\"")),
            "page is missing the shell for card {id}"
        );
        assert!(
            body.contains(&format!("sse-swap=\"{id}\"")),
            "page is missing the sse-swap binding for card {id}"
        );
    }

    // The SSE stream the shells subscribe to.
    assert!(body.contains("sse-connect=\"/events\""));
    // Search is configured, so the header renders a real form.
    assert!(body.contains("action=\"https://search.example.com/search\""));
    // Health names render as neutral placeholder rows before the first probe.
    assert!(body.contains("Prometheus"));
}

// ============================================================================
// GET /card/{id}
// ============================================================================

#[tokio::test]
async fn card_links_renders_the_configured_quick_links() {
    let response = get(app(), "/card/links").await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_string(response).await;

    assert!(body.contains("Quick Links"));
    assert!(body.contains("href=\"http://grafana.example\""));
    assert!(body.contains(">Grafana<"));
    assert!(body.contains("href=\"http://192.168.1.1\""));
    assert!(body.contains(">Router<"));
    assert!(!body.contains("No quick links configured"));
    // The refresh button targets the card's own partial route.
    assert!(body.contains("hx-get=\"/card/links\""));
}

#[tokio::test]
async fn card_unknown_id_is_404() {
    let response = get(app(), "/card/bogus").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_string(response).await, "unknown card");
}

#[tokio::test]
async fn card_ids_are_case_sensitive() {
    let response = get(app(), "/card/Weather").await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(body_string(response).await, "unknown card");
}

/// Fetch failures render inline instead of failing the request. With
/// `HOFVARPNIR_URL` unconfigured the video fetcher errors before it can touch
/// the network, which makes this the offline way to pin that contract.
#[tokio::test]
async fn card_video_renders_the_fetch_error_inline() {
    let response = get(app(), "/card/video").await;

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "a failed fetch must still render a 200 error card"
    );

    let body = body_string(response).await;
    assert!(
        body.contains("Recent Downloads"),
        "header is still rendered"
    );
    // Assert the *contract* — a failed fetch renders `card_error`'s inline body —
    // not the specific message. Which error surfaces depends on the ambient
    // environment: unset `HOFVARPNIR_URL` gives "not configured", while a
    // developer who exports it gets a transport error instead. Both are the
    // error path, and pinning either one would make this test environment-dependent.
    assert!(
        body.contains("text-accent-red"),
        "expected the inline error body from card_error, got: {body}"
    );
}

// ============================================================================
// Static fallback
// ============================================================================

#[tokio::test]
async fn static_files_are_served_from_the_fallback_dir() {
    let dir = scratch_dir("static");
    let css = ":root { --probe: 1 }";
    std::fs::write(dir.join("probe.css"), css).expect("write fixture css");

    let router = build_router(
        AppState {
            config: test_config(),
        },
        dir.to_str().expect("utf-8 temp path"),
    );

    let response = get(router.clone(), "/probe.css").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(header_str(&response, header::CONTENT_TYPE).starts_with("text/css"));
    assert_eq!(body_string(response).await, css);

    let missing = get(router, "/nope").await;
    assert_eq!(missing.status(), StatusCode::NOT_FOUND);

    std::fs::remove_dir_all(&dir).expect("clean up scratch dir");
}

// ============================================================================
// GET /events
// ============================================================================

/// The SSE stream is an infinite loop with a [`dashboard_server::SSE_INTERVAL`]
/// sleep between rounds, so this reads exactly one frame and then drops the
/// body. Awaiting a second round would park the suite for 15s.
#[tokio::test]
async fn events_streams_sse_frames() {
    let response = get(app(), "/events").await;

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        header_str(&response, header::CONTENT_TYPE).starts_with("text/event-stream"),
        "content-type was {:?}",
        header_str(&response, header::CONTENT_TYPE)
    );

    let mut body = response.into_body();

    // Safety net: never wait longer than one SSE round.
    let frame = tokio::time::timeout(Duration::from_secs(10), body.frame())
        .await
        .expect("first SSE frame arrives well inside one SSE_INTERVAL")
        .expect("stream is not exhausted")
        .expect("frame is readable");

    let data = frame.into_data().expect("first frame carries data");
    let text = String::from_utf8(data.to_vec()).expect("frame is utf-8");

    assert!(
        text.starts_with("event: "),
        "expected SSE event framing, got: {text}"
    );
    let event = text
        .lines()
        .next()
        .and_then(|l| l.strip_prefix("event: "))
        .expect("event name");
    assert!(
        dashboard_server::CARD_IDS.contains(&event),
        "unexpected SSE event name {event}"
    );
    assert!(text.contains("\ndata: "), "expected an SSE data line");

    // Drop the stream rather than awaiting the next round.
    drop(body);
}
