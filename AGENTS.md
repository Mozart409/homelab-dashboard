# AGENTS.md - Homelab Dashboard

This document provides guidance for AI coding agents working on this codebase.

## Project Overview

A self-hosted homelab dashboard built with:
- **Server:** Axum 0.8 (async Rust web framework)
- **HTML templating:** Maud (compile-time `html!` macros)
- **Interactivity:** htmx 2.x over a shared Server-Sent Events (SSE) stream
- **Styling:** Tailwind CSS v4
- **Build System:** Nix flakes + crane; `just` + `cargo-watch` for development

## Build/Lint/Test Commands

All day-to-day commands run through `just` (see `justfile`). Run `just` with no
args to list every recipe.

```bash
# Development
just dev                              # Watch: server + Tailwind CSS together
just watch                            # Watch: rebuild + rerun the server only
just run                              # Run the server once (localhost:8080)
just css-watch                        # Watch + rebuild Tailwind CSS only

# Testing
just test                             # Run all tests
just test <name>                      # Run a single test by name (extra args forwarded)
just nextest                          # Run tests with nextest

# Linting & Formatting
just fmt                              # Format code
just fmt-check                        # Check formatting without writing
just clippy                           # Lint (default target)
just pedantic                         # Strict clippy (matches CI gate)

# CSS
just css                              # Build static/dashboard.css once

# Git Hooks (via lefthook, auto-installed by the flake devShell)
lefthook run pre-commit               # keep-sorted + fmt + clippy fix + css build + build
lefthook run commit-msg               # cog verify (conventional commits)
lefthook run pre-push                 # test + fmt check + strict clippy
```

## Project Structure

```
homelab-dashboard/
├── Cargo.toml              # Workspace root (Axum + Maud + htmx deps)
├── justfile                # Dev/test/lint recipes
├── flake.nix               # Nix flake for builds/dev shell
├── cog.toml                # cocogitto (conventional commit) config
├── lefthook.yml            # Git hooks
├── config.toml             # Local config (gitignored)
├── static/
│   ├── input.css           # Tailwind v4 source with theme
│   └── dashboard.css       # Compiled CSS (built by pre-commit hook)
└── crates/
    ├── dashboard-app/      # Shared library crate
    │   └── src/
    │       ├── lib.rs       # DashboardConfig + module wiring
    │       ├── types.rs     # Shared data types (Serialize/Deserialize)
    │       ├── fetch/       # Async service fetchers (API proxies + moka cache)
    │       └── views/       # Maud views (page shell, card partials, search)
    └── dashboard-server/   # Axum binary (main.rs): routes, SSE, config loading
```

## Architecture Notes

- **Workspace:** Two crates — `dashboard-app` (lib) and `dashboard-server` (bin).
- **Rendering:** Pure server-side HTML via Maud. No WASM, no client framework.
- **Cards:** The dashboard is a grid of cards. Each card id (e.g. `weather`,
  `health`) doubles as the SSE event name and the `/card/{id}` route segment.
- **Live updates:** A single SSE stream at `/events` re-renders every card on a
  fixed interval (`SSE_INTERVAL` in `main.rs`); htmx swaps each card's
  `innerHTML` by event name. `/card/{id}` serves the same partial for manual
  refresh.
- **Fetchers:** `fetch/*` are plain async functions that proxy external APIs
  (hiding credentials), with per-service `moka` caches. They return
  `color_eyre::Result<T>`.
- **Views:** `views/*` turn fetched data (or an error) into Maud `Markup`. Fetch
  errors render an inline error body rather than failing the request.
- **Configuration:** `config` crate loads `config.toml` + `DASHBOARD__*` env
  vars in `main.rs`; secrets/URLs are pushed into the environment for the
  fetchers to read (`apply_config_to_env`).

## Code Style Guidelines

### Imports Organization
```rust
use axum::Router;                       // 1. External crates
use std::net::SocketAddr;               // 2. Standard library
use crate::views;                       // 3. Local crate imports
```

SSR-only / heavy deps are often imported inside the function body of a fetcher
(see the fetcher pattern below).

### Naming Conventions

| Element       | Convention             | Example                    |
|---------------|------------------------|----------------------------|
| Modules       | `snake_case`           | `weather.rs`               |
| Fetchers      | `snake_case`           | `get_weather`              |
| View fns      | `snake_case`           | `weather_card`             |
| Functions     | `snake_case`           | `render_card`              |
| Types/Structs | `PascalCase`           | `WeatherData`              |
| Enums         | `PascalCase`           | `HealthStatus::Healthy`    |
| Constants     | `SCREAMING_SNAKE_CASE` | `SSE_INTERVAL`             |

### Error Handling

- **Throughout:** Use `color-eyre` with `Result<T>` and `.wrap_err(...)` for
  context. The server `main` returns `color_eyre::eyre::Result<()>`.
- **Views:** Never propagate fetch errors to the request; render them as an
  inline error card via `card_error(...)`.

```rust
let response: OpenMeteoResponse = client
    .get(&url)
    .send()
    .await
    .wrap_err("Failed to fetch weather")?
    .json()
    .await
    .wrap_err("Failed to parse weather response")?;
```

### Clippy Configuration

Strict settings (the `just pedantic` / pre-push gate):
`-W clippy::pedantic -W clippy::cargo -W clippy::nursery -D warnings`

Common allows:
```rust
#![allow(clippy::multiple_crate_versions)]  // Crate-level (lib.rs / main.rs)
#[allow(clippy::too_many_lines)]            // Function-level when justified
```

### Fetcher Pattern

Async function, body-local imports, `moka` cache in a `LazyLock`, credentials
read from the environment, `color_eyre` errors:

```rust
pub async fn get_weather(lat: f64, lon: f64, location: String) -> Result<WeatherData> {
    use moka::future::Cache;
    use std::sync::LazyLock;
    use std::time::Duration;

    static CACHE: LazyLock<Cache<String, WeatherData>> = LazyLock::new(|| {
        Cache::builder().time_to_live(Duration::from_secs(900)).build()
    });

    let key = format!("{lat:.2},{lon:.2}");
    if let Some(cached) = CACHE.get(&key).await { return Ok(cached); }
    // ... fetch, wrap_err, insert into cache, return
}
```

### View Pattern

A card partial fetches its data and renders `card(...)` on success or
`card_error(...)` on failure:

```rust
pub async fn weather_card(cfg: &DashboardConfig) -> Markup {
    match get_weather(cfg.latitude, cfg.longitude, cfg.location_name.clone()).await {
        Ok(weather) => card("weather", "Weather", None, weather_body(&weather)),
        Err(e) => card_error("weather", "Weather", None, &e.to_string()),
    }
}

fn weather_body(weather: &WeatherData) -> Markup {
    html! { div class="p-4" { /* ... */ } }
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

Use Tailwind utility classes directly in Maud `html!` markup (theme tokens like
`bg-bg-card`, `text-text-primary` are defined in `static/input.css`):
```rust
html! { div class="bg-bg-card border border-border rounded-xl p-4" { /* ... */ } }
```
The pre-commit hook rebuilds `static/dashboard.css` and stages it; for live
editing use `just css-watch` (or `just dev`).

## Common Tasks

### Adding a New Card

1. Add a fetcher in `crates/dashboard-app/src/fetch/new_service.rs`, then
   `pub mod new_service;` and re-export in `fetch/mod.rs`.
2. Add a `new_service_card(...) -> Markup` view in `views/cards.rs`, exported
   from `views/mod.rs`.
3. Register the card id in **all three** places that drive the card list:
   - `CARD_IDS` in `dashboard-server/src/main.rs` (SSE order + route)
   - the `match` in `render_card` (`main.rs`)
   - `CARDS` + the page grid in `views/mod.rs` (`page`)

### Adding a New Fetcher (no card)

1. Create `crates/dashboard-app/src/fetch/new_service.rs`.
2. Add `pub mod new_service;` (and a `pub use` if it should be re-exported) to
   `fetch/mod.rs`.
3. Import heavy/SSR-only deps inside the function body; cache external calls
   with `moka`; read credentials from the environment.
