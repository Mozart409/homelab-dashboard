//! Data fetchers for the various homelab services.
//!
//! These are plain async functions that run on the Axum backend, hiding API
//! keys and providing per-service caching via `moka`. Each returns a
//! [`color_eyre::Result`]; rendering of success/error states is handled by the
//! [`crate::views`] layer.

pub mod health;
pub mod hofvarpnir;
pub mod weather;

pub use health::get_health_overview;
pub use hofvarpnir::get_hofvarpnir_status;
pub use weather::get_weather;
