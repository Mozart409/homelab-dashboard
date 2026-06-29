set shell := ["bash", "-uc"]

set unstable
set dotenv-load

export CARGO_TERM_COLOR := "always"

# List available recipes
default:
    @just --list

# cargo check (default target)
check:
    cargo check

# cargo check on all targets
check-all:
    cargo check --all-targets

# clippy on the default target
clippy:
    cargo clippy

# clippy on all targets
clippy-all:
    cargo clippy --all-targets

# clippy in strict/pedantic mode (matches CI)
pedantic:
    cargo clippy --workspace -- -W clippy::pedantic -W clippy::cargo -W clippy::nursery -D warnings

# Run all tests (pass extra args after `--`, e.g. `just test config::test_default`)
test *ARGS:
    cargo test {{ ARGS }}

# Run tests with nextest
nextest *ARGS:
    cargo nextest run --hide-progress-bar --failure-output final {{ ARGS }}

# Build the docs (no deps)
doc:
    cargo doc --no-deps

# Build and open the docs
doc-open:
    cargo doc --no-deps --open

# Run the server
run *ARGS:
    cargo run -p dashboard-server {{ ARGS }}

# Run server watch + Tailwind CSS watch together (Ctrl-C stops both)
dev:
    #!/usr/bin/env bash
    set -uo pipefail
    trap 'kill 0' EXIT INT TERM
    tailwindcss -i static/input.css -o static/dashboard.css --watch &
    cargo watch -x 'run -p dashboard-server' &
    wait

# Watch sources and restart the server on change
watch:
    cargo watch -x 'run -p dashboard-server'

# Watch sources and re-check on change
watch-check:
    cargo watch -x check

# Watch sources and re-run tests on change
watch-test:
    cargo watch -x test

# Format code
fmt:
    cargo fmt

# Check formatting without writing
fmt-check:
    cargo fmt --check

# Build the Tailwind CSS
css:
    tailwindcss -i static/input.css -o static/dashboard.css

# Watch and rebuild the Tailwind CSS on change
css-watch:
    tailwindcss -i static/input.css -o static/dashboard.css --watch
