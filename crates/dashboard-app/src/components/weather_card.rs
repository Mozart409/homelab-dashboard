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
        <div class="bg-bg-card border border-border rounded-xl overflow-hidden">
            <div class="flex items-center justify-between p-4 border-b border-border-subtle bg-bg-secondary">
                <h3 class="font-mono text-sm font-semibold text-text-primary">"Weather"</h3>
                <button
                    class="flex items-center justify-center w-7 h-7 bg-transparent border border-border rounded text-text-muted cursor-pointer transition-all duration-150 hover:text-text-primary hover:border-text-muted"
                    on:click=move |_| weather_resource.refetch()
                    title="Refresh"
                >
                    <svg class="w-3.5 h-3.5" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                        <path d="M21 12a9 9 0 0 0-9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
                        <path d="M3 3v5h5"/>
                        <path d="M3 12a9 9 0 0 0 9 9 9.75 9.75 0 0 0 6.74-2.74L21 16"/>
                        <path d="M16 21h5v-5"/>
                    </svg>
                </button>
            </div>

            <Suspense fallback=move || view! { <WeatherSkeleton/> }>
                {move || Suspend::new(async move {
                    match weather_resource.await {
                        Ok(weather) => view! { <WeatherContent weather=weather/> }.into_any(),
                        Err(e) => view! {
                            <div class="flex items-center gap-2 p-4 text-accent-red text-sm">
                                <span>"⚠️"</span>
                                <span>{e.to_string()}</span>
                            </div>
                        }.into_any(),
                    }
                })}
            </Suspense>
        </div>
    }
}

#[component]
fn WeatherContent(weather: WeatherData) -> impl IntoView {
    let wind_direction = wind_direction_to_cardinal(weather.wind_direction);

    view! {
        <div class="p-4">
            <div class="flex items-center gap-4 mb-4">
                <span class="text-5xl">{weather.icon.clone()}</span>
                <div class="flex items-start">
                    <span class="font-mono text-4xl font-semibold leading-none">{format!("{:.1}", weather.temperature)}</span>
                    <span class="text-base text-text-muted mt-1">"°C"</span>
                </div>
            </div>

            <div class="text-sm text-text-secondary mb-4">{weather.location.clone()}</div>

            <div class="flex flex-col gap-1 pt-4 border-t border-border-subtle">
                <div class="flex justify-between text-xs">
                    <span class="text-text-muted">"Feels like"</span>
                    <span class="text-text-secondary font-mono">{format!("{:.1}°C", weather.feels_like)}</span>
                </div>
                <div class="flex justify-between text-xs">
                    <span class="text-text-muted">"Humidity"</span>
                    <span class="text-text-secondary font-mono">{format!("{}%", weather.humidity)}</span>
                </div>
                <div class="flex justify-between text-xs">
                    <span class="text-text-muted">"Wind"</span>
                    <span class="text-text-secondary font-mono">
                        {format!("{:.1} km/h {}", weather.wind_speed, wind_direction)}
                    </span>
                </div>
            </div>

            <div class="mt-4 text-xs text-text-muted">
                "Updated: " {weather.fetched_at.format("%H:%M").to_string()}
            </div>
        </div>
    }
}

#[component]
fn WeatherSkeleton() -> impl IntoView {
    view! {
        <div class="p-4 animate-pulse">
            <div class="flex items-center gap-4 mb-4">
                <div class="w-12 h-12 bg-bg-elevated rounded-lg"/>
                <div class="w-20 h-10 bg-bg-elevated rounded"/>
            </div>
            <div class="w-24 h-4 bg-bg-elevated rounded mb-4"/>
            <div class="flex flex-col gap-2 pt-4 border-t border-border-subtle">
                <div class="w-full h-4 bg-bg-elevated rounded"/>
                <div class="w-full h-4 bg-bg-elevated rounded"/>
                <div class="w-full h-4 bg-bg-elevated rounded"/>
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
