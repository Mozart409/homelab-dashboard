//! Custom health check aggregator.

use crate::types::{HealthCheck, HealthOverview, HealthStatus};
use color_eyre::eyre::{Result, WrapErr};

/// Perform health checks on all configured endpoints.
///
/// Reads the check configuration from the `HEALTH_CHECKS` environment variable
/// (a JSON array). Cached for 15 seconds.
#[allow(clippy::too_many_lines)]
pub async fn get_health_overview() -> Result<HealthOverview> {
    use chrono::Utc;
    use moka::future::Cache;
    use serde::Deserialize;
    use std::sync::LazyLock;
    use std::time::Duration;
    use tokio::time::Instant;
    use ulid::Ulid;

    #[derive(Debug, Clone, Deserialize)]
    struct HealthCheckConfig {
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

    async fn check_endpoint(client: &reqwest::Client, config: HealthCheckConfig) -> HealthCheck {
        let id = Ulid::new();
        let start = Instant::now();
        let timeout = Duration::from_millis(config.timeout_ms);

        let result = tokio::time::timeout(timeout, client.get(&config.url).send()).await;

        let (status, response_time_ms, error_message) = match result {
            Ok(Ok(response)) => {
                #[allow(clippy::cast_possible_truncation)]
                let elapsed = start.elapsed().as_millis() as u32;
                let expected = config.expected_status.unwrap_or(200);

                if response.status().as_u16() == expected {
                    (HealthStatus::Healthy, Some(elapsed), None)
                } else {
                    (
                        HealthStatus::Degraded,
                        Some(elapsed),
                        Some(format!("Expected {}, got {}", expected, response.status())),
                    )
                }
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

    // Cache for 15 seconds
    static HEALTH_CACHE: LazyLock<Cache<(), HealthOverview>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(15))
            .max_capacity(1)
            .build()
    });

    if let Some(cached) = HEALTH_CACHE.get(&()).await {
        tracing::debug!("Health cache hit");
        return Ok(cached);
    }

    // Load health check configs from environment
    let checks_json = std::env::var("HEALTH_CHECKS").unwrap_or_else(|_| "[]".to_string());

    let configs: Vec<HealthCheckConfig> =
        serde_json::from_str(&checks_json).wrap_err("Invalid HEALTH_CHECKS config")?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .wrap_err("Failed to create HTTP client")?;

    // Run all health checks in parallel
    let checks: Vec<HealthCheck> = futures::future::join_all(configs.into_iter().map(|config| {
        let client = client.clone();
        async move { check_endpoint(&client, config).await }
    }))
    .await;

    let healthy_count = checks
        .iter()
        .filter(|c| c.status == HealthStatus::Healthy)
        .count();
    let unhealthy_count = checks
        .iter()
        .filter(|c| c.status == HealthStatus::Unhealthy)
        .count();

    let overview = HealthOverview {
        checks,
        healthy_count,
        unhealthy_count,
        fetched_at: Utc::now(),
    };

    HEALTH_CACHE.insert((), overview.clone()).await;
    Ok(overview)
}
