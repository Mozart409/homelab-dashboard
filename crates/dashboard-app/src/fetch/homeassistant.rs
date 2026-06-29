//! Home Assistant API integration.

use crate::types::{HomeAssistantEntity, HomeAssistantStatus};
use color_eyre::eyre::{Result, WrapErr};

/// Fetch Home Assistant status and specified entities.
/// Cached for 30 seconds. An empty `entity_ids` shows all entities.
pub async fn get_homeassistant_status(entity_ids: Vec<String>) -> Result<HomeAssistantStatus> {
    use chrono::{DateTime, Utc};
    use moka::future::Cache;
    use serde::Deserialize;
    use std::sync::LazyLock;
    use std::time::Duration;

    #[derive(Debug, Deserialize)]
    struct ApiConfig {
        version: String,
    }

    #[derive(Debug, Deserialize)]
    struct ApiState {
        entity_id: String,
        state: String,
        attributes: StateAttributes,
        last_changed: DateTime<Utc>,
    }

    #[derive(Debug, Deserialize)]
    struct StateAttributes {
        friendly_name: Option<String>,
        unit_of_measurement: Option<String>,
        icon: Option<String>,
    }

    // Shared cache across requests
    static HA_CACHE: LazyLock<Cache<String, HomeAssistantStatus>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_secs(30))
            .max_capacity(10)
            .build()
    });

    // Cache key based on requested entities
    let cache_key = entity_ids.join(",");

    if let Some(cached) = HA_CACHE.get(&cache_key).await {
        tracing::debug!("Home Assistant cache hit");
        return Ok(cached);
    }

    let base_url =
        std::env::var("HOMEASSISTANT_URL").wrap_err("HOMEASSISTANT_URL not configured")?;
    let token =
        std::env::var("HOMEASSISTANT_TOKEN").wrap_err("HOMEASSISTANT_TOKEN not configured")?;

    let client = reqwest::Client::new();
    let auth_header = format!("Bearer {token}");

    // Fetch HA config for version
    let config: ApiConfig = client
        .get(format!("{base_url}/api/config"))
        .header("Authorization", &auth_header)
        .send()
        .await
        .wrap_err("Failed to fetch HA config")?
        .json()
        .await
        .wrap_err("Failed to parse HA config")?;

    // Fetch all states (we'll filter on our side)
    let all_states: Vec<ApiState> = client
        .get(format!("{base_url}/api/states"))
        .header("Authorization", &auth_header)
        .send()
        .await
        .wrap_err("Failed to fetch HA states")?
        .json()
        .await
        .wrap_err("Failed to parse HA states")?;

    // Filter to requested entities
    let entities: Vec<HomeAssistantEntity> = all_states
        .into_iter()
        .filter(|state| entity_ids.is_empty() || entity_ids.contains(&state.entity_id))
        .map(|state| HomeAssistantEntity {
            entity_id: state.entity_id,
            friendly_name: state
                .attributes
                .friendly_name
                .unwrap_or_else(|| "Unknown".to_string()),
            state: state.state,
            unit: state.attributes.unit_of_measurement,
            icon: state.attributes.icon,
            last_changed: state.last_changed,
        })
        .collect();

    let status = HomeAssistantStatus {
        version: config.version,
        entities,
        fetched_at: Utc::now(),
    };

    HA_CACHE.insert(cache_key, status.clone()).await;
    Ok(status)
}
