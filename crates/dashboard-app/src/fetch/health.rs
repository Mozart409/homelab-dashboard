//! Custom health check aggregator.
//!
//! [`get_health_overview`] is the public entry point: it reads the
//! `HEALTH_CHECKS` environment variable and owns the response cache. The
//! probing itself lives in [`run_checks`], which takes the parsed
//! configuration explicitly and neither reads the environment nor touches the
//! cache, so tests can drive it against a mock server.

use crate::types::{HealthCheck, HealthOverview, HealthStatus};
use chrono::Utc;
use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::Instant;
use ulid::Ulid;

/// One configured endpoint, as it appears in the `HEALTH_CHECKS` JSON array.
#[derive(Debug, Clone, Deserialize)]
pub(crate) struct HealthCheckConfig {
    name: String,
    url: String,
    #[serde(default = "default_timeout")]
    timeout_ms: u64,
    #[serde(default)]
    expected_status: Option<u16>,
}

const fn default_timeout() -> u64 {
    5000
}

/// Decide the status for a response code against the configured expectation.
///
/// The expectation defaults to `200`. The mismatch message renders the
/// received code the way `reqwest` does (`"500 Internal Server Error"`).
fn classify(status: u16, expected: Option<u16>) -> (HealthStatus, Option<String>) {
    let expected = expected.unwrap_or(200);

    if status == expected {
        (HealthStatus::Healthy, None)
    } else {
        let received = reqwest::StatusCode::from_u16(status)
            .map_or_else(|_| status.to_string(), |code| code.to_string());
        (
            HealthStatus::Degraded,
            Some(format!("Expected {expected}, got {received}")),
        )
    }
}

/// Count the healthy and unhealthy checks in an overview.
fn summarize(checks: &[HealthCheck]) -> (usize, usize) {
    let healthy = checks
        .iter()
        .filter(|c| c.status == HealthStatus::Healthy)
        .count();
    let unhealthy = checks
        .iter()
        .filter(|c| c.status == HealthStatus::Unhealthy)
        .count();

    (healthy, unhealthy)
}

async fn check_endpoint(client: &reqwest::Client, config: HealthCheckConfig) -> HealthCheck {
    let id = Ulid::generate();
    let start = Instant::now();
    let timeout = Duration::from_millis(config.timeout_ms);

    let result = tokio::time::timeout(timeout, client.get(&config.url).send()).await;

    let (status, response_time_ms, error_message) = match result {
        Ok(Ok(response)) => {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed = start.elapsed().as_millis() as u32;
            let (status, error_message) =
                classify(response.status().as_u16(), config.expected_status);

            (status, Some(elapsed), error_message)
        }
        Ok(Err(e)) => {
            #[allow(clippy::cast_possible_truncation)]
            let elapsed = start.elapsed().as_millis() as u32;
            (
                HealthStatus::Unhealthy,
                Some(elapsed),
                Some(format!("Request failed: {e}")),
            )
        }
        Err(_) => (
            HealthStatus::Unhealthy,
            None,
            Some(format!("Timeout after {}ms", config.timeout_ms)),
        ),
    };

    HealthCheck {
        id,
        name: config.name,
        url: config.url,
        status,
        response_time_ms,
        last_checked: Utc::now(),
        error_message,
    }
}

/// Build an overview where every configured endpoint is unreachable.
///
/// Used when the HTTP client itself cannot be constructed.
fn unreachable_overview(configs: Vec<HealthCheckConfig>, message: &str) -> HealthOverview {
    let checks: Vec<HealthCheck> = configs
        .into_iter()
        .map(|config| HealthCheck {
            id: Ulid::generate(),
            name: config.name,
            url: config.url,
            status: HealthStatus::Unhealthy,
            response_time_ms: None,
            last_checked: Utc::now(),
            error_message: Some(message.to_string()),
        })
        .collect();

    let (healthy_count, unhealthy_count) = summarize(&checks);

    HealthOverview {
        checks,
        healthy_count,
        unhealthy_count,
        fetched_at: Utc::now(),
    }
}

/// Probe every configured endpoint in parallel and aggregate the results.
///
/// No environment access and no caching — the caller supplies the config.
pub(crate) async fn run_checks(configs: Vec<HealthCheckConfig>) -> HealthOverview {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        // Practically unreachable (TLS backend initialisation). Degrade to an
        // all-unhealthy overview rather than failing the whole card.
        Err(e) => {
            return unreachable_overview(configs, &format!("Failed to create HTTP client: {e}"));
        }
    };

    // Run all health checks in parallel
    let checks: Vec<HealthCheck> = futures::future::join_all(configs.into_iter().map(|config| {
        let client = client.clone();
        async move { check_endpoint(&client, config).await }
    }))
    .await;

    let (healthy_count, unhealthy_count) = summarize(&checks);

    HealthOverview {
        checks,
        healthy_count,
        unhealthy_count,
        fetched_at: Utc::now(),
    }
}

/// Perform health checks on all configured endpoints.
///
/// Reads the check configuration from the `HEALTH_CHECKS` environment variable
/// (a JSON array). Cached for 15 seconds, keyed on that configuration.
pub async fn get_health_overview() -> Result<HealthOverview> {
    use moka::future::Cache;
    use std::sync::LazyLock;

    // Cache for 15 seconds, keyed on the raw config so that a configuration
    // change is visible immediately instead of after the entry expires.
    static HEALTH_CACHE: LazyLock<Cache<String, HealthOverview>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(15))
            .max_capacity(4)
            .build()
    });

    // Load health check configs from environment
    let checks_json = std::env::var("HEALTH_CHECKS").unwrap_or_else(|_| "[]".to_string());

    if let Some(cached) = HEALTH_CACHE.get(&checks_json).await {
        tracing::debug!("Health cache hit");
        return Ok(cached);
    }

    let configs: Vec<HealthCheckConfig> =
        serde_json::from_str(&checks_json).wrap_err("Invalid HEALTH_CHECKS config")?;

    let overview = run_checks(configs).await;

    HEALTH_CACHE.insert(checks_json, overview.clone()).await;
    Ok(overview)
}

#[cfg(test)]
mod tests {
    use super::{
        HealthCheckConfig, classify, default_timeout, run_checks, summarize, unreachable_overview,
    };
    use crate::types::{HealthCheck, HealthStatus};
    use chrono::Utc;
    use std::time::Duration;
    use ulid::Ulid;
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn check(status: HealthStatus) -> HealthCheck {
        HealthCheck {
            id: Ulid::generate(),
            name: "svc".to_string(),
            url: "http://example.invalid".to_string(),
            status,
            response_time_ms: Some(1),
            last_checked: Utc::now(),
            error_message: None,
        }
    }

    fn config(url: String, timeout_ms: u64, expected_status: Option<u16>) -> HealthCheckConfig {
        HealthCheckConfig {
            name: "svc".to_string(),
            url,
            timeout_ms,
            expected_status,
        }
    }

    /// A TCP port with nothing listening on it.
    fn closed_port_url() -> String {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local addr").port();
        drop(listener);
        format!("http://127.0.0.1:{port}")
    }

    // ------------------------------------------------------------------
    // classify
    // ------------------------------------------------------------------

    #[test]
    fn classify_200_without_expectation_is_healthy() {
        assert_eq!(classify(200, None), (HealthStatus::Healthy, None));
    }

    #[test]
    fn classify_matching_explicit_expectation_is_healthy() {
        assert_eq!(classify(204, Some(204)), (HealthStatus::Healthy, None));
    }

    #[test]
    fn classify_200_against_expected_204_is_degraded() {
        let (status, message) = classify(200, Some(204));

        assert_eq!(status, HealthStatus::Degraded);
        assert_eq!(message.as_deref(), Some("Expected 204, got 200 OK"));
    }

    #[test]
    fn classify_500_without_expectation_is_degraded() {
        let (status, message) = classify(500, None);

        assert_eq!(status, HealthStatus::Degraded);
        // The received code is rendered the way `reqwest`'s `StatusCode`
        // displays it — code plus canonical reason, not the bare number.
        assert_eq!(
            message.as_deref(),
            Some("Expected 200, got 500 Internal Server Error")
        );
    }

    #[test]
    fn classify_never_reports_unhealthy() {
        // BUG: a bad status code is only ever `Degraded`, so a service
        // answering 500 is never counted in `unhealthy_count`.
        for code in [301u16, 404, 418, 502] {
            assert_eq!(classify(code, None).0, HealthStatus::Degraded);
        }
    }

    #[test]
    fn classify_unknown_status_code_falls_back_to_the_number() {
        let (_, message) = classify(599, None);
        assert_eq!(
            message.as_deref(),
            Some("Expected 200, got 599 <unknown status code>")
        );
    }

    // ------------------------------------------------------------------
    // summarize
    // ------------------------------------------------------------------

    #[test]
    fn summarize_counts_healthy_and_unhealthy() {
        let checks = vec![
            check(HealthStatus::Healthy),
            check(HealthStatus::Healthy),
            check(HealthStatus::Unhealthy),
        ];

        assert_eq!(summarize(&checks), (2, 1));
    }

    #[test]
    fn summarize_ignores_degraded_and_unknown() {
        // BUG: `Degraded` (and `Unknown`) checks land in neither bucket, so
        // `healthy + unhealthy != checks.len()`. An all-degraded overview
        // reports zero unhealthy services and renders as "All systems
        // operational".
        let checks = vec![
            check(HealthStatus::Degraded),
            check(HealthStatus::Degraded),
            check(HealthStatus::Unknown),
        ];

        let (healthy, unhealthy) = summarize(&checks);
        assert_eq!((healthy, unhealthy), (0, 0));
        assert_ne!(healthy + unhealthy, checks.len());
    }

    #[test]
    fn summarize_of_nothing_is_zero() {
        assert_eq!(summarize(&[]), (0, 0));
    }

    // ------------------------------------------------------------------
    // HealthCheckConfig deserialization
    // ------------------------------------------------------------------

    #[test]
    fn config_defaults_timeout_to_5000ms() {
        let configs: Vec<HealthCheckConfig> =
            serde_json::from_str(r#"[{"name":"a","url":"http://a"}]"#).expect("valid config");

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].timeout_ms, 5000);
        assert_eq!(configs[0].timeout_ms, default_timeout());
    }

    #[test]
    fn config_defaults_expected_status_to_none() {
        let configs: Vec<HealthCheckConfig> =
            serde_json::from_str(r#"[{"name":"a","url":"http://a"}]"#).expect("valid config");

        assert_eq!(configs[0].expected_status, None);
    }

    #[test]
    fn config_reads_explicit_values() {
        let configs: Vec<HealthCheckConfig> = serde_json::from_str(
            r#"[{"name":"a","url":"http://a","timeout_ms":250,"expected_status":204}]"#,
        )
        .expect("valid config");

        assert_eq!(configs[0].name, "a");
        assert_eq!(configs[0].url, "http://a");
        assert_eq!(configs[0].timeout_ms, 250);
        assert_eq!(configs[0].expected_status, Some(204));
    }

    #[test]
    fn config_empty_array_is_empty() {
        let configs: Vec<HealthCheckConfig> = serde_json::from_str("[]").expect("valid config");
        assert!(configs.is_empty());
    }

    #[test]
    fn config_malformed_json_is_rejected() {
        let err = serde_json::from_str::<Vec<HealthCheckConfig>>("{not json}")
            .expect_err("malformed config");
        // The wrapper turns this into `Invalid HEALTH_CHECKS config`.
        let wrapped = color_eyre::eyre::Report::new(err).wrap_err("Invalid HEALTH_CHECKS config");
        assert!(format!("{wrapped:?}").contains("Invalid HEALTH_CHECKS config"));
    }

    #[test]
    fn config_missing_url_is_rejected() {
        assert!(serde_json::from_str::<Vec<HealthCheckConfig>>(r#"[{"name":"a"}]"#).is_err());
    }

    // ------------------------------------------------------------------
    // unreachable_overview
    // ------------------------------------------------------------------

    #[test]
    fn unreachable_overview_marks_everything_unhealthy() {
        let overview = unreachable_overview(
            vec![config("http://a".to_string(), 100, None)],
            "Failed to create HTTP client: boom",
        );

        assert_eq!(overview.checks.len(), 1);
        assert_eq!(overview.checks[0].status, HealthStatus::Unhealthy);
        assert_eq!(overview.unhealthy_count, 1);
        assert_eq!(overview.healthy_count, 0);
    }

    // ------------------------------------------------------------------
    // run_checks (mock HTTP)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn run_checks_without_config_is_empty() {
        let overview = run_checks(vec![]).await;

        assert!(overview.checks.is_empty());
        assert_eq!(overview.healthy_count, 0);
        assert_eq!(overview.unhealthy_count, 0);
    }

    #[tokio::test]
    async fn run_checks_reports_a_200_as_healthy() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let overview = run_checks(vec![config(server.uri(), 2000, None)]).await;

        assert_eq!(overview.checks.len(), 1);
        assert_eq!(overview.checks[0].status, HealthStatus::Healthy);
        assert_eq!(overview.checks[0].error_message, None);
        assert!(overview.checks[0].response_time_ms.is_some());
        assert_eq!(overview.healthy_count, 1);
        assert_eq!(overview.unhealthy_count, 0);
    }

    #[tokio::test]
    async fn run_checks_honours_a_matching_expected_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&server)
            .await;

        let overview = run_checks(vec![config(server.uri(), 2000, Some(204))]).await;

        assert_eq!(overview.checks[0].status, HealthStatus::Healthy);
        assert_eq!(overview.healthy_count, 1);
    }

    #[tokio::test]
    async fn run_checks_degrades_on_a_mismatched_expected_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let overview = run_checks(vec![config(server.uri(), 2000, Some(204))]).await;

        assert_eq!(overview.checks[0].status, HealthStatus::Degraded);
        assert_eq!(
            overview.checks[0].error_message.as_deref(),
            Some("Expected 204, got 200 OK")
        );
        // BUG: degraded checks are counted nowhere.
        assert_eq!(overview.healthy_count, 0);
        assert_eq!(overview.unhealthy_count, 0);
    }

    #[tokio::test]
    async fn run_checks_reports_a_refused_connection_as_unhealthy() {
        let overview = run_checks(vec![config(closed_port_url(), 5000, None)]).await;

        assert_eq!(overview.checks[0].status, HealthStatus::Unhealthy);
        assert!(
            overview.checks[0]
                .error_message
                .as_deref()
                .expect("error message")
                .starts_with("Request failed: "),
            "unexpected message: {:?}",
            overview.checks[0].error_message
        );
        assert_eq!(overview.unhealthy_count, 1);
    }

    #[tokio::test]
    async fn run_checks_reports_a_slow_endpoint_as_a_timeout() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_millis(500)))
            .mount(&server)
            .await;

        let overview = run_checks(vec![config(server.uri(), 50, None)]).await;

        assert_eq!(overview.checks[0].status, HealthStatus::Unhealthy);
        assert_eq!(overview.checks[0].response_time_ms, None);
        assert_eq!(
            overview.checks[0].error_message.as_deref(),
            Some("Timeout after 50ms")
        );
        assert_eq!(overview.unhealthy_count, 1);
    }

    #[tokio::test]
    async fn run_checks_probes_every_endpoint() {
        let healthy = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&healthy)
            .await;

        let degraded = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&degraded)
            .await;

        let overview = run_checks(vec![
            config(healthy.uri(), 2000, None),
            config(degraded.uri(), 2000, None),
            config(closed_port_url(), 2000, None),
        ])
        .await;

        assert_eq!(overview.checks.len(), 3);
        assert_eq!(overview.checks[0].status, HealthStatus::Healthy);
        assert_eq!(overview.checks[1].status, HealthStatus::Degraded);
        assert_eq!(overview.checks[2].status, HealthStatus::Unhealthy);
        assert_eq!(overview.healthy_count, 1);
        assert_eq!(overview.unhealthy_count, 1);
    }
}
