{
  description = "Homelab Dashboard - An Axum + Maud + htmx dashboard for self-hosted services";

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
          config = {
            allowUnfree = true;
          };
        };

        rustToolchain = pkgs.rust-bin.stable."1.96.1".default;

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
          ];
        };

        # Build the cargo dependencies
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Build CSS with Tailwind v4
        tailwindCss = pkgs.stdenv.mkDerivation {
          pname = "dashboard-css";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;

          nativeBuildInputs = [pkgs.tailwindcss_4];

          buildPhase = ''
            tailwindcss -i static/input.css -o static/dashboard.css --minify
          '';

          installPhase = ''
            mkdir -p $out
            cp static/dashboard.css $out/
          '';
        };

        # Build the server binary
        server = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;

            pname = "dashboard-server";

            cargoExtraArgs = "-p dashboard-server";

            postInstall = ''
              mkdir -p $out/share/dashboard

              # Copy Tailwind-compiled CSS
              cp ${tailwindCss}/dashboard.css $out/share/dashboard/

              # Copy other static assets if any (excluding input.css)
              if [ -d static ]; then
                for f in static/*; do
                  if [ "$(basename "$f")" != "input.css" ]; then
                    cp -r "$f" $out/share/dashboard/
                  fi
                done
              fi
            '';
          });
      in {
        packages = {
          default = server;
          inherit server tailwindCss;
        };

        devShells.default = craneLib.devShell {
          # Inherit inputs from commonArgs
          inputsFrom = [server];

          packages = with pkgs; [
            # keep-sorted start
            cargo-audit
            cargo-deny
            cargo-edit
            cargo-watch
            claude-code
            cocogitto
            just
            keep-sorted
            lefthook
            lldb
            opencode
            playwright-driver.browsers
            rust-analyzer
            rustToolchain
            tailwindcss_4
            # keep-sorted end
          ];

          shellHook = ''
            lefthook install
            export PLAYWRIGHT_BROWSERS_PATH=${pkgs.playwright-driver.browsers}
            export PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS=true
            echo "🏠 Homelab Dashboard Development Shell"
            echo ""
            echo "Commands:"
            echo "  just                 - List all available recipes"
            echo "  just dev             - Watch: server + Tailwind CSS together"
            echo "  just watch           - Watch: rebuild + rerun the server"
            echo "  just run             - Run the server"
            echo "  just test            - Run tests"
            echo "  just fmt             - Format code"
            echo "  just clippy          - Lint code"
            echo "  just css-watch       - Watch + rebuild Tailwind CSS"
            echo ""
          '';
        };
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
