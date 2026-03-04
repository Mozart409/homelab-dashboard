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
        <Stylesheet href="/pkg/dashboard.css"/>

        <main class="dashboard">
            <header class="dashboard-header">
                <h1 class="dashboard-title">"🏠 Homelab"</h1>
                <div class="search-wrapper">
                    <SearchBox searxng_url=searxng_url/>
                </div>
            </header>

            <div class="dashboard-grid">
                // Top row: Weather + Recent videos
                <section class="grid-section top-row">
                    <WeatherCard config=weather_config/>
                    <VideoCard/>
                </section>

                // Middle row: Service cards
                <section class="grid-section services-row">
                    <ProxmoxCard/>
                    <JellyfinCard/>
                    <HomeAssistantCard config=ha_config/>
                </section>

                // Bottom row: Health checks
                <section class="grid-section health-row">
                    <HealthGrid/>
                </section>
            </div>

            <footer class="dashboard-footer">
                <span>"Homelab Dashboard"</span>
                <span class="separator">"•"</span>
                <span>"Powered by Leptos + Axum"</span>
            </footer>
        </main>
    }
}
