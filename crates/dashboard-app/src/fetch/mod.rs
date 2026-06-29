//! Data fetchers for the various homelab services.
//!
//! These are plain async functions that run on the Axum backend, hiding API
//! keys and providing per-service caching via `moka`. Each returns a
//! [`color_eyre::Result`]; rendering of success/error states is handled by the
//! [`crate::views`] layer.

pub mod health;
pub mod hofvarpnir;
pub mod homeassistant;
pub mod jellyfin;
pub mod proxmox;
pub mod weather;

pub use health::get_health_overview;
pub use hofvarpnir::get_hofvarpnir_status;
pub use homeassistant::get_homeassistant_status;
pub use jellyfin::get_jellyfin_status;
pub use proxmox::get_proxmox_status;
pub use weather::get_weather;
