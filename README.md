# Homelab Dashboard

A self-hosted dashboard for monitoring homelab services, built with **Leptos** and **Axum** in Rust.

![Dashboard Preview](docs/preview.png)

## Features

- 🔍 **SearXNG Integration** - Search the web via your self-hosted instance
- 🌤️ **Weather** - Current conditions from Open-Meteo (free, no API key)
- 📺 **Hofvarpnir** - Recent video downloads from your archival system
- 🩺 **Health Checks** - Monitor service availability

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Leptos App (SSR + Hydration)             │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Dashboard UI (WASM)                                  │  │
│  │  Search │ Weather │ Videos │ Services │ Health        │  │
│  └───────────────────────────────────────────────────────┘  │
│                              │                              │
│  ┌───────────────────────────▼───────────────────────────┐  │
│  │  Axum Backend (Server Functions)                      │  │
│  │  • Caching with moka                                  │  │
│  │  • API token management                               │  │
│  │  • Parallel service fetching                          │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

### Prerequisites

- The yggdrasil root dev shell (stable Rust, Tailwind, cargo tools); run the `hdash-*` recipes from the repo root
- [cargo-leptos](https://github.com/leptos-rs/cargo-leptos)

```bash
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos
```

### Development

```bash
# Clone and enter directory
git clone https://github.com/yourusername/homelab-dashboard
cd homelab-dashboard

# Copy and edit config
cp config.example.toml config.toml
# Edit config.toml with your service URLs and API keys

# Run development server with hot reload
cargo leptos watch
```

Open http://localhost:8080

### Production Build

```bash
cargo leptos build --release
```

## NixOS Installation

### In yggdrasil

The root flake builds the package (`nix build .#homelab-dashboard`, from
`nix/default.nix` with the shared Rust toolchain) and exports
`nixosModules.homelab-dashboard`, which brings its own overlay. A host imports
it through `self` (see `infra/hosts/containers/homelab-dashboard`):

```nix
{self, ...}: {
  imports = [self.nixosModules.homelab-dashboard];
  services.homelab-dashboard = {
    enable = true;
    settings = {
      listen_address = "127.0.0.1";
      port = 8084;
    };
  };
}
```

### Secrets File

Create `/run/secrets/dashboard-env` (or use sops-nix/agenix):

```bash
HOFVARPNIR_API_KEY=optional-api-key
```

## Configuration

Configuration can be provided via:

1. `config.toml` in the working directory
2. `/etc/homelab-dashboard/config.toml`
3. Environment variables with `DASHBOARD__` prefix (e.g., `DASHBOARD__PORT=3001`)

See [config.example.toml](config.example.toml) for all options.

## Service Setup

### Hofvarpnir

The dashboard integrates with the [Hofvarpnir](http://192.168.2.100:3000/docs) video archival system API.

Used endpoints:
- `GET /api/v1/downloads?status=Completed` — recent completed downloads
- `GET /api/v1/system/status` — download statistics

## Development

### Project Structure

```
homelab-dashboard/
├── crates/
│   ├── dashboard-app/     # Leptos frontend + server functions
│   │   └── src/
│   │       ├── components/  # UI components
│   │       ├── server/      # Server functions (API proxies)
│   │       └── types.rs     # Shared types
│   └── dashboard-server/  # Axum binary
├── static/                # CSS and static assets
└── nix/
    ├── default.nix       # crane build, called by the root flake
    └── module.nix        # NixOS module
```

### Adding a New Service Card

1. Add types in `crates/dashboard-app/src/types.rs`
2. Create server function in `crates/dashboard-app/src/server/`
3. Create component in `crates/dashboard-app/src/components/`
4. Add to `App` in `crates/dashboard-app/src/app.rs`
5. Add config to `nix/module.nix` if needed

## Tech Stack

- **Frontend**: [Leptos](https://leptos.dev/) (Rust → WASM)
- **Backend**: [Axum](https://github.com/tokio-rs/axum)
- **Caching**: [moka](https://github.com/moka-rs/moka)
- **Error Handling**: [color-eyre](https://github.com/yaahc/color-eyre)
- **IDs**: [ulid](https://github.com/dylanhart/ulid-rs)

## License

MIT
