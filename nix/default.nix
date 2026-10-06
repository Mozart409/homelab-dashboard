# Package for rust/homelab-dashboard, called from the root flake:
#
#   import ./rust/homelab-dashboard/nix {inherit pkgs crane toolchain;}
#
# `toolchain` is the monorepo's one stable Rust (rust/toolchain.nix `build`).
# Returns `server` (dashboard-server plus its static assets under
# share/dashboard), `tailwindCss` and `cargoArtifacts`.
{
  pkgs,
  crane,
  toolchain,
}: let
  craneLib = (crane.mkLib pkgs).overrideToolchain toolchain;

  # Only the Cargo sources, so editing the README or the Nix files does not
  # rebuild the server.
  commonArgs = {
    src = craneLib.cleanCargoSource ../.;
    strictDeps = true;
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  # Tailwind v4 scans the `@source` paths in static/input.css (the .rs
  # templates), so it gets the static/ and crates/ trees.
  tailwindCss = pkgs.stdenv.mkDerivation {
    pname = "dashboard-css";
    version = "0.1.0";
    src = pkgs.lib.fileset.toSource {
      root = ../.;
      fileset = pkgs.lib.fileset.unions [../static ../crates];
    };
    nativeBuildInputs = [pkgs.tailwindcss_4];
    buildPhase = ''
      tailwindcss -i static/input.css -o static/dashboard.css --minify
    '';
    installPhase = ''
      mkdir -p $out
      cp static/dashboard.css $out/
    '';
  };

  server = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
      pname = "dashboard-server";
      cargoExtraArgs = "-p dashboard-server";
      meta.mainProgram = "dashboard-server";
      postInstall = ''
        mkdir -p $out/share/dashboard
        cp ${tailwindCss}/dashboard.css $out/share/dashboard/
      '';
    });
in {
  inherit server tailwindCss cargoArtifacts;
}
