{
  description = "Homelab Dashboard - A Leptos + Axum dashboard for self-hosted services";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    crane = {
      url = "github:ipetkov/crane";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = {
    self,
    nixpkgs,
    rust-overlay,
    crane,
    flake-utils,
    ...
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import rust-overlay)];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        # Rust toolchain with WASM target
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          targets = ["wasm32-unknown-unknown"];
        };

        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Common build inputs
        commonArgs = {
          src = craneLib.cleanCargoSource ./.;

          strictDeps = true;

          buildInputs = with pkgs;
            [
              openssl
              pkg-config
            ]
            ++ lib.optionals stdenv.isDarwin [
              darwin.apple_sdk.frameworks.Security
              darwin.apple_sdk.frameworks.SystemConfiguration
            ];

          nativeBuildInputs = with pkgs; [
            pkg-config
            wasm-bindgen-cli
            binaryen # For wasm-opt
          ];
        };

        # Build the cargo dependencies
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build the WASM client
        wasmClient = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;

            pname = "dashboard-app-wasm";

            cargoExtraArgs = "-p dashboard-app --target wasm32-unknown-unknown --features hydrate";

            # Don't run tests for WASM
            doCheck = false;

            postBuild = ''
              wasm-bindgen \
                --target web \
                --out-dir $out/pkg \
                --out-name dashboard \
                target/wasm32-unknown-unknown/release/dashboard_app.wasm

              # Optimize WASM
              wasm-opt -Oz -o $out/pkg/dashboard_bg.wasm $out/pkg/dashboard_bg.wasm
            '';
          });

        # Build the server binary
        server = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;

            pname = "dashboard-server";

            cargoExtraArgs = "-p dashboard-server";

            postInstall = ''
              # Copy WASM assets
              mkdir -p $out/share/dashboard/pkg
              cp -r ${wasmClient}/pkg/* $out/share/dashboard/pkg/

              # Copy static assets if any
              if [ -d static ]; then
                cp -r static/* $out/share/dashboard/
              fi
            '';
          });
      in {
        packages = {
          default = server;
          inherit server wasmClient;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from commonArgs
          inputsFrom = [server];

          packages = with pkgs; [
            # Rust tools
            rustToolchain
            rust-analyzer
            cargo-watch
            cargo-leptos

            # WASM tools
            wasm-bindgen-cli
            wasm-pack
            binaryen

            # Development tools
            trunk # Alternative to cargo-leptos for dev

            # Debugging
            lldb

            # AI
            opencode

            # CSS
            tailwindcss_4
          ];

          shellHook = ''
            echo "🏠 Homelab Dashboard Development Shell"
            echo ""
            echo "Commands:"
            echo "  cargo leptos watch  - Run dev server with hot reload"
            echo "  cargo build         - Build server"
            echo "  cargo test          - Run tests"
            echo ""
          '';
        };

        # NixOS module
        nixosModules.default = import ./module.nix;
      }
    )
    // {
      # Cross-system outputs
      nixosModules.default = import ./module.nix;

      overlays.default = final: prev: {
        homelab-dashboard = self.packages.${prev.system}.default;
      };
    };
}
