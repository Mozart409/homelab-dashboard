//! Weather data fetching from Open-Meteo API.
//!
//! [`get_weather`] is the public entry point: it owns the response cache and
//! the default Open-Meteo base URL. All HTTP work lives in [`fetch_weather`],
//! which takes an explicit base URL and neither reads the environment nor
//! touches the cache, so tests can point it at a mock server.

use crate::types::{WeatherCondition, WeatherData};
use chrono::Utc;
use color_eyre::eyre::{Result, WrapErr};
use serde::Deserialize;

/// Base URL of the public Open-Meteo API.
const OPEN_METEO_BASE_URL: &str = "https://api.open-meteo.com";

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

/// Cache key for a coordinate pair, rounded to two decimal places.
fn cache_key(latitude: f64, longitude: f64) -> String {
    format!("{latitude:.2},{longitude:.2}")
}

/// Build the Open-Meteo `current` forecast URL for a coordinate pair.
fn build_url(base_url: &str, latitude: f64, longitude: f64) -> String {
    format!(
        "{base_url}/v1/forecast?latitude={latitude}&longitude={longitude}&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,wind_direction_10m,weather_code&timezone=auto"
    )
}

/// Map an Open-Meteo response onto the dashboard's [`WeatherData`].
fn to_weather_data(response: &OpenMeteoResponse, location_name: String) -> WeatherData {
    let condition = WeatherCondition::from_wmo_code(response.current.weather_code);

    WeatherData {
        temperature: response.current.temperature_2m,
        feels_like: response.current.apparent_temperature,
        humidity: response.current.relative_humidity_2m,
        wind_speed: response.current.wind_speed_10m,
        wind_direction: response.current.wind_direction_10m,
        icon: condition.icon().to_string(),
        condition,
        location: location_name,
        fetched_at: Utc::now(),
    }
}

/// Fetch current weather from an Open-Meteo compatible host.
///
/// No environment access and no caching — the caller supplies everything.
pub(crate) async fn fetch_weather(
    base_url: &str,
    latitude: f64,
    longitude: f64,
    location_name: String,
) -> Result<WeatherData> {
    let url = build_url(base_url, latitude, longitude);

    // Bounded like the health checker's client: an unbounded request would stall
    // the SSE render loop for as long as the OS keeps the socket open.
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .wrap_err("Failed to create HTTP client")?;
    let response: OpenMeteoResponse = client
        .get(&url)
        .send()
        .await
        .wrap_err("Failed to fetch weather")?
        .json()
        .await
        .wrap_err("Failed to parse weather response")?;

    Ok(to_weather_data(&response, location_name))
}

/// Fetch current weather data from Open-Meteo.
/// Cached for 15 minutes on the server side.
pub async fn get_weather(
    latitude: f64,
    longitude: f64,
    location_name: String,
) -> Result<WeatherData> {
    use moka::future::Cache;
    use std::sync::LazyLock;
    use std::time::Duration;

    // Shared cache across requests
    static WEATHER_CACHE: LazyLock<Cache<String, WeatherData>> = LazyLock::new(|| {
        Cache::builder()
            .time_to_live(Duration::from_mins(15)) // 15 minutes
            .max_capacity(10)
            .build()
    });

    // Cache key based on coordinates (rounded to 2 decimal places)
    let cache_key = cache_key(latitude, longitude);

    // Check cache first
    if let Some(cached) = WEATHER_CACHE.get(&cache_key).await {
        tracing::debug!("Weather cache hit for {}", cache_key);
        return Ok(cached);
    }

    tracing::info!("Fetching weather for {location_name} ({latitude}, {longitude})");

    let weather_data =
        fetch_weather(OPEN_METEO_BASE_URL, latitude, longitude, location_name).await?;

    // Store in cache
    WEATHER_CACHE.insert(cache_key, weather_data.clone()).await;

    Ok(weather_data)
}

#[cfg(test)]
mod tests {
    use super::{
        CurrentWeather, OpenMeteoResponse, build_url, cache_key, fetch_weather, to_weather_data,
    };
    use crate::types::WeatherCondition;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    const BODY: &str = r#"{
        "current": {
            "temperature_2m": 21.4,
            "apparent_temperature": 19.8,
            "relative_humidity_2m": 63,
            "wind_speed_10m": 12.5,
            "wind_direction_10m": 245,
            "weather_code": 61
        }
    }"#;

    fn approx(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() < 1e-9
    }

    // ------------------------------------------------------------------
    // cache_key
    // ------------------------------------------------------------------

    #[test]
    fn cache_key_rounds_to_two_decimals() {
        assert_eq!(cache_key(52.521, 13.409), "52.52,13.41");
        assert_eq!(cache_key(52.529, 13.401), "52.53,13.40");
    }

    #[test]
    fn cache_key_collides_within_rounding_window() {
        // BUG: coordinates up to ~0.005 degrees apart (≈550 m) share a cache
        // entry, so nearby locations silently serve each other's weather.
        assert_eq!(cache_key(52.521, 13.4), cache_key(52.524, 13.4));
    }

    #[test]
    fn cache_key_handles_negative_coordinates() {
        assert_eq!(cache_key(-33.8688, -151.2093), "-33.87,-151.21");
    }

    #[test]
    fn cache_key_keeps_negative_zero() {
        // `-0.0` formats with its sign, so it is a *different* key from `0.0`
        // even though the two compare equal.
        assert_eq!(cache_key(-0.0, 0.0), "-0.00,0.00");
        assert_ne!(cache_key(-0.0, 0.0), cache_key(0.0, 0.0));
    }

    // ------------------------------------------------------------------
    // build_url
    // ------------------------------------------------------------------

    #[test]
    fn build_url_contains_every_required_current_param() {
        let url = build_url("https://api.open-meteo.com", 52.52, 13.41);

        assert!(url.starts_with("https://api.open-meteo.com/v1/forecast?"));
        assert!(url.contains("latitude=52.52"));
        assert!(url.contains("longitude=13.41"));
        assert!(url.contains("timezone=auto"));

        for param in [
            "temperature_2m",
            "apparent_temperature",
            "relative_humidity_2m",
            "wind_speed_10m",
            "wind_direction_10m",
            "weather_code",
        ] {
            assert!(url.contains(param), "missing current param: {param}");
        }
    }

    #[test]
    fn build_url_does_not_round_coordinates() {
        // Unlike the cache key, the request keeps full precision.
        let url = build_url("http://localhost", 52.521, 13.405);
        assert!(url.contains("latitude=52.521"));
        assert!(url.contains("longitude=13.405"));
    }

    // ------------------------------------------------------------------
    // to_weather_data
    // ------------------------------------------------------------------

    #[test]
    fn to_weather_data_maps_every_field() {
        let response: OpenMeteoResponse = serde_json::from_str(BODY).expect("fixture parses");
        let data = to_weather_data(&response, "Berlin".to_string());

        assert!(approx(data.temperature, 21.4));
        assert!(approx(data.feels_like, 19.8));
        assert_eq!(data.humidity, 63);
        assert!(approx(data.wind_speed, 12.5));
        assert_eq!(data.wind_direction, 245);
        assert_eq!(data.condition, WeatherCondition::Rain);
        assert_eq!(data.icon, WeatherCondition::Rain.icon());
        assert_eq!(data.location, "Berlin");
    }

    #[test]
    fn to_weather_data_derives_icon_from_condition() {
        let response = OpenMeteoResponse {
            current: CurrentWeather {
                temperature_2m: 0.0,
                apparent_temperature: 0.0,
                relative_humidity_2m: 0,
                wind_speed_10m: 0.0,
                wind_direction_10m: 0,
                weather_code: 0,
            },
        };
        let data = to_weather_data(&response, String::new());

        assert_eq!(data.condition, WeatherCondition::Clear);
        assert_eq!(data.icon, WeatherCondition::Clear.icon());
    }

    #[test]
    fn humidity_above_u8_range_fails_to_parse() {
        let body = BODY.replace(
            "\"relative_humidity_2m\": 63",
            "\"relative_humidity_2m\": 300",
        );
        let err =
            serde_json::from_str::<OpenMeteoResponse>(&body).expect_err("300 does not fit in a u8");

        assert!(
            err.to_string().contains("invalid value"),
            "unexpected error: {err}"
        );
    }

    // ------------------------------------------------------------------
    // fetch_weather (mock HTTP)
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn fetch_weather_returns_mapped_data_on_200() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/forecast"))
            .and(query_param("latitude", "52.52"))
            .and(query_param("longitude", "13.41"))
            .and(query_param("timezone", "auto"))
            .respond_with(ResponseTemplate::new(200).set_body_string(BODY))
            .expect(1)
            .mount(&server)
            .await;

        let data = fetch_weather(&server.uri(), 52.52, 13.41, "Berlin".to_string())
            .await
            .expect("mock server responds with valid JSON");

        assert!(approx(data.temperature, 21.4));
        assert_eq!(data.humidity, 63);
        assert_eq!(data.condition, WeatherCondition::Rain);
        assert_eq!(data.location, "Berlin");
    }

    #[tokio::test]
    async fn fetch_weather_sends_the_url_built_by_build_url() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string(BODY))
            .mount(&server)
            .await;

        fetch_weather(&server.uri(), 52.52, 13.41, "Berlin".to_string())
            .await
            .expect("request succeeds");

        let requests = server
            .received_requests()
            .await
            .expect("request recording is enabled");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].url.path(), "/v1/forecast");

        // The `current=` list travels with literal commas — `build_url` does no
        // percent-encoding and Open-Meteo accepts it as-is.
        let query = requests[0].url.query().expect("query string present");
        assert_eq!(
            query,
            "latitude=52.52&longitude=13.41&current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,wind_direction_10m,weather_code&timezone=auto"
        );
    }

    #[tokio::test]
    async fn fetch_weather_500_is_an_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;

        let err = fetch_weather(&server.uri(), 1.0, 2.0, "Nowhere".to_string())
            .await
            .expect_err("a 500 must not yield weather data");

        // NOTE: `reqwest` does not treat a 5xx status as a transport error, so
        // the failure surfaces at the JSON decode step ("Failed to parse
        // weather response"), not at "Failed to fetch weather". The latter is
        // reserved for connect/transport failures.
        assert!(
            format!("{err:?}").contains("Failed to parse weather response"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_weather_transport_failure_says_failed_to_fetch() {
        // Nothing is listening on this port, so `send()` itself fails.
        let err = fetch_weather("http://127.0.0.1:1", 1.0, 2.0, "Nowhere".to_string())
            .await
            .expect_err("connection refused");

        assert!(
            format!("{err:?}").contains("Failed to fetch weather"),
            "unexpected error: {err:?}"
        );
    }

    #[tokio::test]
    async fn fetch_weather_malformed_body_is_a_parse_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{ not json"))
            .mount(&server)
            .await;

        let err = fetch_weather(&server.uri(), 1.0, 2.0, "Nowhere".to_string())
            .await
            .expect_err("malformed body must not yield weather data");

        assert!(
            format!("{err:?}").contains("Failed to parse weather response"),
            "unexpected error: {err:?}"
        );
    }
}
