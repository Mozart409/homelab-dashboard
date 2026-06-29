//! Weather data fetching from Open-Meteo API.

use crate::types::{WeatherCondition, WeatherData};
use color_eyre::eyre::{Result, WrapErr};

/// Fetch current weather data from Open-Meteo.
/// Cached for 15 minutes on the server side.
pub async fn get_weather(
    latitude: f64,
    longitude: f64,
    location_name: String,
) -> Result<WeatherData> {
    use chrono::Utc;
    use moka::future::Cache;
    use serde::Deserialize;
    use std::sync::LazyLock;
    use std::time::Duration;

    /// Open-Meteo API response structure
    #[derive(Debug, Deserialize)]
    struct OpenMeteoResponse {
        current: CurrentWeather,
    }

    #[derive(Debug, Deserialize)]
    struct CurrentWeather {
        temperature_2m: f64,
        apparent_temperature: f64,
        relative_humidity_2m: u8,
        wind_speed_10m: f64,
        wind_direction_10m: u16,
        weather_code: u8,
    }

    // Shared cache across requests
    static WEATHER_CACHE: LazyLock<Cache<String, WeatherData>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_mins(15)) // 15 minutes
            .max_capacity(10)
            .build()
    });

    // Cache key based on coordinates (rounded to 2 decimal places)
    let cache_key = format!("{latitude:.2},{longitude:.2}");

    // Check cache first
    if let Some(cached) = WEATHER_CACHE.get(&cache_key).await {
        tracing::debug!("Weather cache hit for {}", cache_key);
        return Ok(cached);
    }

    tracing::info!("Fetching weather for {location_name} ({latitude}, {longitude})");

    let url = format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude}&longitude={longitude}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,wind_direction_10m,weather_code&timezone=auto"
    );

    let client = reqwest::Client::new();
    let response: OpenMeteoResponse = client
        .get(&url)
        .send()
        .await
        .wrap_err("Failed to fetch weather")?
        .json()
        .await
        .wrap_err("Failed to parse weather response")?;

    let condition = WeatherCondition::from_wmo_code(response.current.weather_code);

    let weather_data = WeatherData {
        temperature: response.current.temperature_2m,
        feels_like: response.current.apparent_temperature,
        humidity: response.current.relative_humidity_2m,
        wind_speed: response.current.wind_speed_10m,
        wind_direction: response.current.wind_direction_10m,
        icon: condition.icon().to_string(),
        condition,
        location: location_name,
        fetched_at: Utc::now(),
    };

    // Store in cache
    WEATHER_CACHE.insert(cache_key, weather_data.clone()).await;

    Ok(weather_data)
}
