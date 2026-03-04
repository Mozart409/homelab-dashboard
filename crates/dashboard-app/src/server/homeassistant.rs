//! Home Assistant API integration.

use crate::types::{HomeAssistantEntity, HomeAssistantStatus};
use chrono::{DateTime, Utc};
use leptos::prelude::*;
use serde::Deserialize;

// ============================================================================
// Home Assistant API Response Types
// ============================================================================

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

// ============================================================================
// Server Functions
// ============================================================================

/// Fetch Home Assistant status and specified entities.
#[server]
pub async fn get_homeassistant_status(
    entity_ids: Vec<String>,
) -> Result<HomeAssistantStatus, ServerFnError> {
    use std::sync::LazyLock;
    use std::time::Duration;
    use moka::future::Cache;

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

    let base_url = std::env::var("HOMEASSISTANT_URL")
        .map_err(|_| ServerFnError::new("HOMEASSISTANT_URL not configured"))?;
    let token = std::env::var("HOMEASSISTANT_TOKEN")
        .map_err(|_| ServerFnError::new("HOMEASSISTANT_TOKEN not configured"))?;

    let client = reqwest::Client::new();
    let auth_header = format!("Bearer {token}");

    // Fetch HA config for version
    let config: ApiConfig = client
        .get(format!("{base_url}/api/config"))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch HA config: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse HA config: {e}")))?;

    // Fetch all states (we'll filter on our side)
    let all_states: Vec<ApiState> = client
        .get(format!("{base_url}/api/states"))
        .header("Authorization", &auth_header)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch HA states: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse HA states: {e}")))?;

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

/// Fetch a single entity's state.
#[server]
pub async fn get_entity_state(entity_id: String) -> Result<HomeAssistantEntity, ServerFnError> {
    let base_url = std::env::var("HOMEASSISTANT_URL")
        .map_err(|_| ServerFnError::new("HOMEASSISTANT_URL not configured"))?;
    let token = std::env::var("HOMEASSISTANT_TOKEN")
        .map_err(|_| ServerFnError::new("HOMEASSISTANT_TOKEN not configured"))?;

    let client = reqwest::Client::new();
    
    let state: ApiState = client
        .get(format!("{base_url}/api/states/{entity_id}"))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to fetch entity {entity_id}: {e}")))?
        .json()
        .await
        .map_err(|e| ServerFnError::new(format!("Failed to parse entity state: {e}")))?;

    Ok(HomeAssistantEntity {
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
}
