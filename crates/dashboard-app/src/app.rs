//! Main application component.

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, Html, Meta, Stylesheet, Title};

use crate::components::{
    HealthGrid, HomeAssistantCard, HomeAssistantConfig, JellyfinCard, ProxmoxCard, SearchBox,
    VideoCard, WeatherCard, WeatherConfig,
};

/// Root application component.
#[component]
#[allow(clippy::must_use_candidate)]
pub fn App() -> impl IntoView {
    // Provide meta context for SSR
    provide_meta_context();

    // Configuration signals (in production, load from server state)
    let searxng_url = Signal::derive(|| {
        option_env!("SEARXNG_URL")
            .unwrap_or("https://search.example.com")
            .to_string()
    });

    let weather_config = WeatherConfig {
        latitude: option_env!("WEATHER_LAT")
            .and_then(|s| s.parse().ok())
            .unwrap_or(52.52), // Berlin default
        longitude: option_env!("WEATHER_LON")
            .and_then(|s| s.parse().ok())
            .unwrap_or(13.41),
        location_name: option_env!("WEATHER_LOCATION")
            .unwrap_or("Berlin")
            .to_string(),
    };

    let ha_config = HomeAssistantConfig {
        entity_ids: vec![
            // Add your preferred entity IDs here
            // "sensor.living_room_temperature".to_string(),
            // "binary_sensor.front_door".to_string(),
        ],
    };

    view! {
        <Html attr:lang="en"/>
        <Meta charset="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        <Title text="Homelab Dashboard"/>
        <Stylesheet href="/dashboard.css"/>

        <main class="max-w-[1400px] mx-auto p-6 min-h-screen flex flex-col bg-bg-primary text-text-primary font-sans">
            <header class="flex items-center justify-between gap-6 mb-8 pb-6 border-b border-border flex-col md:flex-row">
                <h1 class="font-mono text-2xl font-semibold text-text-primary flex items-center gap-2">"🏠 Homelab"</h1>
                <div class="flex-1 max-w-[500px] w-full md:w-auto">
                    <SearchBox searxng_url=searxng_url/>
                </div>
            </header>

            <div class="flex-1 flex flex-col gap-8">
                // Top row: Weather + Recent videos
                <section class="grid gap-6 grid-cols-1 lg:grid-cols-[300px_1fr]">
                    <WeatherCard config=weather_config/>
                    <VideoCard/>
                </section>

                // Middle row: Service cards
                <section class="grid gap-6 grid-cols-1 md:grid-cols-2 xl:grid-cols-3">
                    <ProxmoxCard/>
                    <JellyfinCard/>
                    <HomeAssistantCard config=ha_config/>
                </section>

                // Bottom row: Health checks
                <section class="grid gap-6">
                    <HealthGrid/>
                </section>
            </div>

            <footer class="mt-auto pt-6 border-t border-border-subtle flex items-center justify-center gap-2 text-text-muted text-sm">
                <span>"Homelab Dashboard"</span>
                <span>"•"</span>
                <span>"Powered by Leptos + Axum"</span>
            </footer>
        </main>
    }
}
