# AGENTS.md - Homelab Dashboard

This document provides guidance for AI coding agents working on this codebase.

## Project Overview

A self-hosted homelab dashboard built with:
- **Frontend/Backend:** Leptos 0.8 (full-stack Rust web framework with SSR + hydration)
- **HTTP Server:** Axum
- **Styling:** Tailwind CSS v4
- **Build System:** Nix flakes + crane, cargo-leptos for development

## Build/Lint/Test Commands

```bash
# Development
cargo leptos watch                    # Dev server with hot reload (localhost:3000)
cargo leptos build --release          # Production build

# Testing
cargo test --workspace                # Run all tests
cargo test <test_name>                # Run single test by name
cargo test -p dashboard-app <name>    # Run test in specific crate

# Linting & Formatting
cargo fmt                             # Format code
cargo clippy --workspace -- -W clippy::pedantic -W clippy::cargo -W clippy::nursery -D warnings

# CSS
tailwindcss -i static/input.css -o static/dashboard.css

# Git Hooks (via lefthook)
lefthook run pre-commit               # fmt + clippy fix + build
lefthook run pre-push                 # test + fmt check + strict clippy
```

## Project Structure

```
homelab-dashboard/
├── Cargo.toml              # Workspace root with cargo-leptos config
├── flake.nix               # Nix flake for builds/dev
├── config.toml             # Local config (gitignored)
├── static/
│   ├── input.css           # Tailwind v4 source with theme
│   └── dashboard.css       # Compiled CSS
└── crates/
    ├── dashboard-app/      # Leptos frontend + server functions
    │   └── src/
    │       ├── lib.rs, app.rs, types.rs
    │       ├── components/ # UI components
    │       └── server/     # Server functions (API proxies)
    └── dashboard-server/   # Axum binary (main.rs)
```

## Code Style Guidelines

### Imports Organization
```rust
use axum::Router;                    // 1. External crates
use std::net::SocketAddr;            // 2. Standard library
use crate::components::WeatherCard;  // 3. Local crate imports
```

### Naming Conventions

| Element       | Convention             | Example                    |
|---------------|------------------------|----------------------------|
| Modules       | `snake_case`           | `weather_card.rs`          |
| Components    | `PascalCase`           | `WeatherCard`              |
| Functions     | `snake_case`           | `get_weather`              |
| Types/Structs | `PascalCase`           | `WeatherData`              |
| Enums         | `PascalCase`           | `HealthStatus::Healthy`    |
| Constants     | `SCREAMING_SNAKE_CASE` | `DEFAULT_TIMEOUT`          |

### Error Handling

- **Server binary:** Use `color-eyre` with `Result<(), eyre::Report>`
- **Server functions:** Use `ServerFnError` with `.map_err(|e| ServerFnError::new(...))?`

```rust
#[server]
pub async fn get_data() -> Result<Data, ServerFnError> {
    client.get(url).send().await
        .map_err(|e| ServerFnError::new(format!("Request failed: {e}")))?;
}
```

### Clippy Configuration

Strict settings: `-W clippy::pedantic -W clippy::cargo -W clippy::nursery -D warnings`

Common allows:
```rust
#![allow(clippy::multiple_crate_versions)]  // Crate-level in lib.rs
#[allow(clippy::too_many_lines)]            // Function-level when justified
```

### Server Functions Pattern

Import SSR-only dependencies inside the function body, use `moka` for caching:

```rust
#[server]
pub async fn get_weather() -> Result<WeatherData, ServerFnError> {
    use moka::sync::Cache;
    use std::sync::LazyLock;

    static CACHE: LazyLock<Cache<String, WeatherData>> = LazyLock::new(|| {
        Cache::builder().time_to_live(Duration::from_secs(300)).build()
    });

    if let Some(cached) = CACHE.get(&key) { return Ok(cached); }
    // ... fetch and cache
}
```

### Component Pattern

```rust
#[component]
pub fn WeatherCard(config: WeatherConfig) -> impl IntoView {
    let weather = Resource::new(|| (), |_| get_weather());

    view! {
        <Suspense fallback=move || view! { <LoadingSkeleton /> }>
            {move || weather.get().map(|result| match result {
                Ok(data) => view! { <WeatherDisplay data=data /> }.into_any(),
                Err(e) => view! { <ErrorCard message=e.to_string() /> }.into_any(),
            })}
        </Suspense>
    }
}
```

### Types and Serialization

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceStatus {
    pub name: String,
    pub healthy: bool,
}
```

Use ULIDs for IDs (not UUIDs).

### CSS / Styling

Use Tailwind utility classes directly in Leptos `view!` macros:
```rust
view! { <div class="bg-gray-900 rounded-lg p-4 shadow-lg">...</div> }
```

## Architecture Notes

- **Workspace:** Two crates - `dashboard-app` (lib) and `dashboard-server` (bin)
- **Compilation targets:** `dashboard-app` compiles to WASM (hydrate) and native (SSR)
- **Server functions:** Proxy external APIs to hide credentials from frontend
- **Caching:** Use `moka` with `LazyLock<Cache<K, V>>` for static cache instances
- **Configuration:** TOML files + environment variables (see `config.example.toml`)

## Common Tasks

### Adding a New Component
1. Create `crates/dashboard-app/src/components/new_component.rs`
2. Add `pub mod new_component;` to `components/mod.rs`
3. Export with `pub use new_component::NewComponent;`

### Adding a New Server Function
1. Create `crates/dashboard-app/src/server/new_service.rs`
2. Add `pub mod new_service;` to `server/mod.rs`
3. Import SSR-only deps inside the function body
4. Use `moka` caching for external API calls
