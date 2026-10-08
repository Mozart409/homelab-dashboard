{
  description = "Homelab Dashboard - an Axum + Maud + htmx dashboard for self-hosted services";

  # In the yggdrasil monorepo this is a subflake of the root flake, which makes
  # every input below follow its own, so the toolchain is the monorepo's one
  # stable Rust (rust/toolchain.nix `build` there). The monorepo itself builds
  # the project by importing ./nix directly with its shared toolchain; this file
  # exists so the exported GitHub repo builds without the monorepo around it.
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = inputs @ {
    self,
    flake-parts,
    ...
  }: let
    # Latest stable, pinned by flake.lock rather than a version literal.
    # Keep the component list equal to `build` in yggdrasil's
    # rust/toolchain.nix (the root's `rust-toolchain-sync` asserts it).
    toolchainFor = system:
      inputs.fenix.packages.${system}.stable.withComponents [
        "cargo"
        "clippy"
        "rust-src"
        "rustc"
        "rustfmt"
      ];

    dashboardFor = pkgs:
      import ./nix {
        inherit pkgs;
        inherit (inputs) crane;
        toolchain = toolchainFor pkgs.stdenv.hostPlatform.system;
      };
  in
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin"];

      perSystem = {
        pkgs,
        system,
        ...
      }: let
        dashboard = dashboardFor pkgs;
        toolchain = toolchainFor system;
      in {
        packages = {
          default = dashboard.server;
          inherit (dashboard) server tailwindCss;
          inherit toolchain;
        };

        checks.server = dashboard.server;

        devShells.default = pkgs.mkShell {
          packages = with pkgs; [
            # keep-sorted start
            cargo-deny
            cargo-nextest
            cargo-watch
            inputs.fenix.packages.${system}.stable.rust-analyzer
            tailwindcss_4
            toolchain
            # keep-sorted end
          ];
        };
      };

      flake = {
        # The overlay builds against the consumer's nixpkgs, with this flake's
        # pinned toolchain and crane.
        overlays.default = final: _prev: {
          homelab-dashboard = (dashboardFor final).server;
        };

        # `default` brings the overlay that defines pkgs.homelab-dashboard;
        # `homelab-dashboard` is the bare module (set `package` yourself).
        nixosModules = {
          default = {
            imports = [./nix/module.nix];
            nixpkgs.overlays = [self.overlays.default];
          };
          homelab-dashboard = ./nix/module.nix;
        };
      };
    };
}
