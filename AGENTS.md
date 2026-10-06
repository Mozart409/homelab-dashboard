# AGENTS.md - Homelab Dashboard

This document provides guidance for AI coding agents working on this codebase.

## Project Overview

A self-hosted homelab dashboard built with:
- **Server:** Axum 0.8 (async Rust web framework)
- **HTML templating:** Maud (compile-time `html!` macros)
- **Interactivity:** htmx 2.x over a shared Server-Sent Events (SSE) stream
- **Styling:** Tailwind CSS v4
- **Build System:** crane via the yggdrasil root flake (`nix/default.nix`, shared toolchain `rust/toolchain.nix`); root `just` recipes + `cargo-watch` for development

## Build/Lint/Test Commands

This project lives in the yggdrasil monorepo and has no tooling of its own:
the dev shell, `justfile`, lefthook and cog are the root ones. Run recipes from
the repo root (`just --list | grep hdash`).

```bash
# Development
just hdash-dev                        # Build the CSS, then rerun the server on changes
just hdash-run                        # Build the CSS, run the server once (localhost:8080)
just hdash-css-watch                  # Watch + rebuild Tailwind CSS only
just hdash-css                        # Build static/dashboard.css once

# Testing
just hdash-test                       # Run all tests
just hdash-test <name>                # Run a single test by name (extra args forwarded)
just hdash-nextest                    # Run tests with nextest

# Linting & Formatting
just hdash-fmt                        # Format code
just hdash-fmt-check                  # Check formatting without writing
just hdash-clippy                     # Strict clippy (pedantic/cargo/nursery, -D warnings)
just hdash-deny                       # cargo deny
just hdash-ci                         # fmt check + clippy + tests + deny (the old pre-push gate)

# Nix (from the repo root)
nix build .#homelab-dashboard         # The package the `containers` host runs
```

The root hooks do not run cargo: run `just hdash-ci` before pushing.

## Project Structure

```
homelab-dashboard/
├── Cargo.toml              # Workspace root (Axum + Maud + htmx deps)
├── nix/
│   ├── default.nix         # crane build, called by the root flake
│   └── module.nix          # NixOS module (root: nixosModules.homelab-dashboard)
├── config.toml             # Local config (gitignored)
├── static/
│   ├── input.css           # Tailwind v4 source with theme
│   └── dashboard.css       # Compiled CSS for `cargo run` (just hdash-css); Nix builds its own
└── crates/
    ├── dashboard-app/      # Shared library crate
    │   └── src/
    │       ├── lib.rs       # DashboardConfig + module wiring
    │       ├── types.rs     # Shared data types (Serialize/Deserialize)
    │       ├── fetch/       # Async service fetchers (API proxies + moka cache)
    │       └── views/       # Maud views (page shell, card partials, search)
    └── dashboard-server/   # Axum server
        ├── src/lib.rs       # Routes, SSE, config loading, build_router
        ├── src/main.rs      # Thin binary: setup -> build_router -> serve
        └── tests/           # Route integration tests (tower::oneshot)
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
- **Fetchers:** `fetch/*` are split in two. A **public wrapper** owns the
  environment reads and the `moka` cache; it delegates to a `pub(crate)` **inner
  fn** that takes the base URL / config explicitly and does no env access and no
  caching. Tests target the inner fn against a `wiremock` server, so they need no
  environment variables and touch no global state. Both return
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

Strict settings (`just hdash-clippy`, part of `just hdash-ci`):
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
editing use `just hdash-css-watch` (or `just hdash-dev`).

## Common Tasks

### Adding a New Card

1. Add a fetcher in `crates/dashboard-app/src/fetch/new_service.rs`, then
   `pub mod new_service;` and re-export in `fetch/mod.rs`.
2. Add a `new_service_card(...) -> Markup` view in `views/cards.rs`, exported
   from `views/mod.rs`.
3. Register the card id in **all three** places that drive the card list:
   - `CARD_IDS` in `dashboard-server/src/lib.rs` (SSE order + route)
   - the `match` in `render_card` (`lib.rs`)
   - `CARDS` + the page grid in `views/mod.rs` (`page`)

   A unit test in `dashboard-server/src/lib.rs` asserts these lists agree as
   sets, so missing one of the three fails the suite rather than silently
   dropping the card.

### Adding a New Fetcher (no card)

1. Create `crates/dashboard-app/src/fetch/new_service.rs`.
2. Add `pub mod new_service;` (and a `pub use` if it should be re-exported) to
   `fetch/mod.rs`.
3. Import heavy/SSR-only deps inside the function body; cache external calls
   with `moka`; read credentials from the environment.

### Committing Changes

Commits follow Conventional Commits, enforced by `cog verify` (commit-msg hook).

1. Inspect the tree (`git status`, `git diff`) and recent `git log` to match the
   existing style.
2. Split changes into logical units; stage per unit (hunk-level staging when one
   file spans several units) so each commit is self-contained.
3. Commit in the house style — a short, lowercase `type(scope): summary`, e.g.
   `feat(links): add quick links card`. Types: `feat`, `fix`, `refactor`,
   `chore`, `docs`, `ci`, `perf`.
4. The root `pre-commit` hook runs alejandra, keep-sorted, `just --fmt` and
   shellcheck only; run `just hdash-fmt` (and `just hdash-css` after template
   changes) yourself before committing.
5. Run `git` from the real shell, never a sandboxed subprocess: signing needs an
   askpass prompt the sandbox can't provide.
6. Leave the tree clean and run `just hdash-ci` (tests, fmt check, strict
   clippy, `cargo deny`) before pushing; no hook does it for you.
