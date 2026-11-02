# What `just hdash-ci` compiles (fmt, clippy, tests) as crane checks, called
# from the yggdrasil root flake; `cargo deny` stays a plain recipe (it fetches
# the advisory database, and a build has no network):
#
#   import ./rust/homelab-dashboard/nix/checks.nix {inherit pkgs crane toolchain;}
#
# As Nix builds they are cached by their inputs: a push that does not change
# the source finds them already in the store, and the rust-ci pre-push gate
# (the root's legacyPackages.rust-ci.homelab-dashboard) costs an evaluation
# instead of a compile. The standalone flake does not use this file.
#
# Returns `checks` and `cargoArtifacts`.
{
  pkgs,
  crane,
  toolchain,
}: let
  inherit (pkgs) lib;
  inherit (import ./. {inherit pkgs crane toolchain;}) commonArgs craneLib;

  checkArgs =
    commonArgs
    // {
      # The package's Cargo sources plus the fixtures the route tests read.
      src = lib.fileset.toSource {
        root = ../.;
        fileset = lib.fileset.unions [
          (craneLib.fileset.commonCargoSources ../.)
          ../crates/dashboard-server/tests/fixtures
        ];
      };
      pname = "homelab-dashboard";
      version = "0.0.0";
      # The dev profile `cargo test` uses, so the tests keep debug assertions
      # and overflow checks (crane, and the package, default to release).
      CARGO_PROFILE = "dev";
    };

  cargoArtifacts = craneLib.buildDepsOnly checkArgs;
in {
  inherit cargoArtifacts;

  checks = {
    fmt = craneLib.cargoFmt {
      inherit (checkArgs) src;
      pname = "homelab-dashboard-fmt";
    };

    clippy = craneLib.cargoClippy (checkArgs
      // {
        inherit cargoArtifacts;
        pname = "homelab-dashboard-clippy";
        # Kept equal to `just hdash-clippy`.
        cargoClippyExtraArgs = "--workspace -- -W clippy::pedantic -W clippy::cargo -W clippy::nursery -D warnings";
      });

    test = craneLib.cargoTest (checkArgs
      // {
        inherit cargoArtifacts;
        pname = "homelab-dashboard-test";
        cargoTestExtraArgs = "--workspace";
        # reqwest's rustls verifier loads the system roots when a client is
        # built, and the sandbox has none ("builder error" in the fetch tests).
        SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
      });
  };
}
