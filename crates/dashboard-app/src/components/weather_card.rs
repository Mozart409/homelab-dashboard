//! Weather card component displaying current conditions from Open-Meteo.

use leptos::prelude::*;
use crate::server::get_weather;
use crate::types::WeatherData;

/// Weather card configuration
#[derive(Clone)]
pub struct WeatherConfig {
    pub latitude: f64,
    pub longitude: f64,
    pub location_name: String,
}

/// Displays current weather conditions.
#[component]
pub fn WeatherCard(config: WeatherConfig) -> impl IntoView {
    let weather_resource = Resource::new(
        || (),
        move |()| {
            let cfg = config.clone();
            async move {
                get_weather(cfg.latitude, cfg.longitude, cfg.location_name).await
            }
        },
    );

    view! {
        <div class="card weather-card">
            <div class="card-header">
                <h3 class="card-title">"Weather"</h3>
                <button
                    class="refresh-btn"
                    on:click=move |_| weather_resource.refetch()
                    title="Refresh"
                >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
                        <path d="M3 3v5h5"/>
                        <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/>
                        <path d="M16 21h5v-5"/>
                    </svg>
                </button>
            </div>

            <Suspense fallback=move || view! { <WeatherSkeleton/> }>
                {move || {
                    weather_resource.get().map(|result| {
                        match result {
                            Ok(weather) => view! { <WeatherContent weather=weather/> }.into_any(),
                            Err(e) => view! {
                                <div class="card-error">
                                    <span class="error-icon">"⚠️"</span>
                                    <span>{e.to_string()}</span>
                                </div>
                            }.into_any(),
                        }
                    })
                }}
            </Suspense>
        </div>
    }
}

#[component]
fn WeatherContent(weather: WeatherData) -> impl IntoView {
    let wind_direction = wind_direction_to_cardinal(weather.wind_direction);

    view! {
        <div class="weather-content">
            <div class="weather-main">
                <span class="weather-icon">{weather.icon.clone()}</span>
                <div class="weather-temp">
                    <span class="temp-value">{format!("{:.1}", weather.temperature)}</span>
                    <span class="temp-unit">"°C"</span>
                </div>
            </div>

            <div class="weather-location">{weather.location.clone()}</div>

            <div class="weather-details">
                <div class="weather-detail">
                    <span class="detail-label">"Feels like"</span>
                    <span class="detail-value">{format!("{:.1}°C", weather.feels_like)}</span>
                </div>
                <div class="weather-detail">
                    <span class="detail-label">"Humidity"</span>
                    <span class="detail-value">{format!("{}%", weather.humidity)}</span>
                </div>
                <div class="weather-detail">
                    <span class="detail-label">"Wind"</span>
                    <span class="detail-value">
                        {format!("{:.1} km/h {}", weather.wind_speed, wind_direction)}
                    </span>
                </div>
            </div>

            <div class="weather-updated">
                "Updated: " {weather.fetched_at.format("%H:%M").to_string()}
            </div>
        </div>
    }
}

#[component]
fn WeatherSkeleton() -> impl IntoView {
    view! {
        <div class="weather-content skeleton">
            <div class="weather-main">
                <div class="skeleton-icon"/>
                <div class="skeleton-temp"/>
            </div>
            <div class="skeleton-location"/>
            <div class="weather-details">
                <div class="skeleton-detail"/>
                <div class="skeleton-detail"/>
                <div class="skeleton-detail"/>
            </div>
        </div>
    }
}

fn wind_direction_to_cardinal(degrees: u16) -> &'static str {
    match degrees {
        0..=22 | 338..=360 => "N",
        23..=67 => "NE",
        68..=112 => "E",
        113..=157 => "SE",
        158..=202 => "S",
        203..=247 => "SW",
        248..=292 => "W",
        293..=337 => "NW",
        _ => "",
    }
}
