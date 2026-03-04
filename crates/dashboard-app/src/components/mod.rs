//! UI components for the homelab dashboard.

mod health_grid;
mod search_box;
mod service_card;
mod video_card;
mod weather_card;

pub use health_grid::HealthGrid;
pub use search_box::SearchBox;
pub use service_card::{HomeAssistantCard, HomeAssistantConfig, JellyfinCard, ProxmoxCard};
pub use video_card::VideoCard;
pub use weather_card::{WeatherCard, WeatherConfig};
