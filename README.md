# Homelab Dashboard

A self-hosted dashboard for monitoring homelab services, built with **Leptos** and **Axum** in Rust.

![Dashboard Preview](docs/preview.png)

## Features

- 🔍 **SearXNG Integration** - Search the web via your self-hosted instance
- 🌤️ **Weather** - Current conditions from Open-Meteo (free, no API key)
- 📺 **Hofvarpnir** - Recent video downloads from your archival system
- 🖥️ **Proxmox** - Node and VM status overview
- 🎬 **Jellyfin** - Media library stats and active streams
- 🏠 **Home Assistant** - Entity states and sensors
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

- Rust 1.75+ with `wasm32-unknown-unknown` target
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

### Using Flakes

```nix
# flake.nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    homelab-dashboard.url = "github:yourusername/homelab-dashboard";
  };

  outputs = { self, nixpkgs, homelab-dashboard, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        homelab-dashboard.nixosModules.default
        ({ pkgs, ... }: {
          nixpkgs.overlays = [ homelab-dashboard.overlays.default ];

          services.homelab-dashboard = {
            enable = true;

            settings = {
              listen_address = "0.0.0.0";
              port = 8080;

              search = {
                type = "searxng";
                url = "https://search.example.com";
              };

              weather = {
                latitude = 52.52;
                longitude = 13.41;
                location = "Berlin";
              };

              proxmox = {
                url = "https://proxmox.local:8006";
                token_id = "dashboard@pve!monitoring";
                # Use secretsFile for token_secret
              };

              jellyfin.url = "https://jellyfin.local";
              homeassistant.url = "https://ha.local";
              hofvarpnir.url = "https://hofvarpnir.local";

              health_checks = [
                { name = "Router"; url = "http://192.168.1.1"; }
                { name = "NAS"; url = "http://nas.local:5000"; }
              ];
            };

            # Load secrets from a file
            secretsFile = "/run/secrets/dashboard-env";

            openFirewall = true;
          };
        })
      ];
    };
  };
}
```

### Secrets File

Create `/run/secrets/dashboard-env` (or use sops-nix/agenix):

```bash
PROXMOX_TOKEN_SECRET=xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
JELLYFIN_API_KEY=your-jellyfin-api-key
HOMEASSISTANT_TOKEN=your-long-lived-access-token
HOFVARPNIR_API_KEY=optional-api-key
```

## Configuration

Configuration can be provided via:

1. `config.toml` in the working directory
2. `/etc/homelab-dashboard/config.toml`
3. Environment variables with `DASHBOARD__` prefix (e.g., `DASHBOARD__PORT=3001`)

See [config.example.toml](config.example.toml) for all options.

## Service Setup

### Proxmox

1. Create an API token: **Datacenter → Permissions → API Tokens → Add**
2. Use format: `user@realm!tokenid` for `token_id`
3. Grant `PVEAuditor` role or custom role with:
   - `Sys.Audit`
   - `VM.Audit`

### Jellyfin

1. Go to **Dashboard → API Keys → Add**
2. Copy the generated key to config

### Home Assistant

1. Go to **Profile → Long-Lived Access Tokens → Create Token**
2. Copy the token to config

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
├── flake.nix             # Nix flake
└── module.nix            # NixOS module
```

### Adding a New Service Card

1. Add types in `crates/dashboard-app/src/types.rs`
2. Create server function in `crates/dashboard-app/src/server/`
3. Create component in `crates/dashboard-app/src/components/`
4. Add to `App` in `crates/dashboard-app/src/app.rs`
5. Add config to `module.nix` if needed

## Tech Stack

- **Frontend**: [Leptos](https://leptos.dev/) (Rust → WASM)
- **Backend**: [Axum](https://github.com/tokio-rs/axum)
- **Caching**: [moka](https://github.com/moka-rs/moka)
- **Error Handling**: [color-eyre](https://github.com/yaahc/color-eyre)
- **IDs**: [ulid](https://github.com/dylanhart/ulid-rs)

## License

MIT
