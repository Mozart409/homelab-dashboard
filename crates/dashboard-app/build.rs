//! Embed build metadata (git hash + build time) as compile-time env vars.
//!
//! The values are read in the footer view via `env!`. The git hash degrades
//! gracefully so builds without a `.git` checkout (e.g. the Nix sandbox) still
//! succeed, falling back to `unknown`.

use std::process::Command;

use chrono::Utc;

fn main() {
    // Git short hash: allow an explicit override (e.g. injected by Nix), else
    // ask git, else fall back to a placeholder.
    let git_hash = std::env::var("GIT_HASH")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            Command::new("git")
                .args(["rev-parse", "--short", "HEAD"])
                .output()
                .ok()
                .filter(|out| out.status.success())
                .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
                .filter(|s| !s.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_owned());

    // Actual compile time (the footer answers "when was this built?").
    let build_time = Utc::now().format("%Y-%m-%d %H:%M UTC").to_string();

    println!("cargo:rustc-env=GIT_HASH={git_hash}");
    println!("cargo:rustc-env=BUILD_TIME={build_time}");

    // Rebuild when the checked-out commit moves or the override changes.
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    println!("cargo:rerun-if-env-changed=GIT_HASH");
}
