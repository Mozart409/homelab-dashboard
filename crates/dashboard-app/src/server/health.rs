//! Custom health check aggregator.

use crate::types::{HealthCheck, HealthOverview, HealthStatus};
use chrono::Utc;
use leptos::prelude::*;
use serde::Deserialize;
use std::time::Duration;
use ulid::Ulid;

// ============================================================================
// Configuration Types
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
pub struct HealthCheckConfig {
    pub name: String,
    pub url: String,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub expected_status: Option<u16>,
}

fn default_timeout() -> u64 {
    5000
}

// ============================================================================
// Server Functions
// ============================================================================

/// Perform health checks on all configured endpoints.
#[server]
pub async fn get_health_overview() -> Result<HealthOverview, ServerFnError> {
    use std::sync::LazyLock;
    use moka::future::Cache;

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
    let checks_json = std::env::var("HEALTH_CHECKS")
        .unwrap_or_else(|_| "[]".to_string());
    
    let configs: Vec<HealthCheckConfig> = serde_json::from_str(&checks_json)
        .map_err(|e| ServerFnError::new(format!("Invalid HEALTH_CHECKS config: {e}")))?;

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| ServerFnError::new(format!("Failed to create HTTP client: {e}")))?;

    // Run all health checks in parallel
    let checks: Vec<HealthCheck> = futures::future::join_all(
        configs.into_iter().map(|config| {
            let client = client.clone();
            async move {
                check_endpoint(&client, config).await
            }
        })
    )
    .await;

    let healthy_count = checks.iter().filter(|c| c.status == HealthStatus::Healthy).count();
    let unhealthy_count = checks.iter().filter(|c| c.status == HealthStatus::Unhealthy).count();

    let overview = HealthOverview {
        checks,
        healthy_count,
        unhealthy_count,
        fetched_at: Utc::now(),
    };

    HEALTH_CACHE.insert((), overview.clone()).await;
    Ok(overview)
}

async fn check_endpoint(client: &reqwest::Client, config: HealthCheckConfig) -> HealthCheck {
    use tokio::time::Instant;

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

/// Check a single endpoint (for manual refresh).
#[server]
pub async fn check_single_endpoint(name: String, url: String) -> Result<HealthCheck, ServerFnError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| ServerFnError::new(format!("Failed to create HTTP client: {e}")))?;

    let config = HealthCheckConfig {
        name,
        url,
        timeout_ms: 5000,
        expected_status: None,
    };

    Ok(check_endpoint(&client, config).await)
}
