//! Server-side functions for fetching data from various services.
//!
//! These are Leptos server functions that run on the Axum backend,
//! hiding API keys and providing caching.

pub mod health;
pub mod hofvarpnir;
pub mod homeassistant;
pub mod jellyfin;
pub mod proxmox;
pub mod weather;

// Re-export server functions for easier imports
pub use health::{check_single_endpoint, get_health_overview};
pub use hofvarpnir::get_hofvarpnir_status;
pub use homeassistant::{get_entity_state, get_homeassistant_status};
pub use jellyfin::get_jellyfin_status;
pub use proxmox::get_proxmox_status;
pub use weather::get_weather;
