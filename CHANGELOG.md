# Changelog
All notable changes to this project will be documented in this file. See [conventional commits](https://www.conventionalcommits.org/) for commit guidelines.

- - -
## v0.2.0 - 2026-07-31
#### Features
- (**deps**) upgrade cargo deps - (c0afb77) - Amadeus Mader
#### Refactoring
- (**tests**) refactor for testing - (63cf209) - Amadeus Mader

- - -

## v0.1.0 - 2026-07-31
#### Features
- (**cog**) move release workflow to cog - (e65562e) - Amadeus Mader
- (**css**) add tailwindcss_4 - (52dc8ff) - Amadeus Mader
- (**favicon**) add favicon - (aebda40) - Amadeus Mader
- (**footer**) show version, git hash, and build time - (c1fe48c) - Amadeus Mader
- (**footer**) add github link - (906dcaf) - Amadeus Mader
- (**health**) render service names before first probe - (fd6c0a2) - Amadeus Mader
- (**health**) surface configured check names in config - (d3f1357) - Amadeus Mader
- (**layout**) change layout and update order - (4eb0753) - Amadeus Mader
- (**links**) add quick links card - (52a759d) - Amadeus Mader
- (**nix**) add quick_links option - (d2f18ea) - Amadeus Mader
- (**search**) redirect queries to searxng instance - (66ca56a) - Amadeus Mader
- (**video**) show 6 recent downloads - (63a4294) - Amadeus Mader
- integrate Pinchflat API for recent downloads - (cded0bd) - Amadeus Mader
- add caching to service integrations - (4405d7e) - Amadeus Mader
- add Cargo.lock - (65c49d4) - Amadeus Mader
- initial homelab dashboard project - (bc1c3fe) - Amadeus Mader
#### Bug Fixes
- (**css**) only watch rust files not target/ dir - (8d65a82) - Amadeus Mader
- (**css**) cache bust css - (400c4f0) - Amadeus Mader
- (**css**) remove unused classes - (12e8630) - Amadeus Mader
- (**downloads**) add custom_name - (0fdc6df) - Amadeus Mader
- (**nix**) correct tailwind css derivation output path - (0e3606a) - Amadeus Mader
- (**nix**) strip null values before generating toml config - (1b24f83) - Amadeus Mader
- (**quicklinks**) render links - (c614fab) - Amadeus Mader
- (**views**) morph card swaps to preserve dom nodes - (f73b7ba) - Amadeus Mader
- deny clippy warnings in pre-commit hook - (e5bca6f) - Amadeus Mader
- remove redundant clone on video.platform - (573754d) - Amadeus Mader
- add claude to git ignore - (24e0a0d) - Amadeus Mader
#### Performance Improvements
- (**sse**) push card frames only when changed - (3f045d3) - Amadeus Mader
#### Documentation
- (**agents**) add git commit workflow - (295a5fb) - Amadeus Mader
- (**agents**) rewrite for axum and maud stack - (7599176) - Amadeus Mader
#### Continuous Integration
- (**hooks**) auto-stage compiled css in build step - (32a9806) - Amadeus Mader
- (**hooks**) verify commits, sort files, and build css - (43fa06a) - Amadeus Mader
#### Refactoring
- (**cards**) remove proxmox, jellyfin and homeassistant - (f1423ac) - Amadeus Mader
- (**css**) migrate from custom CSS to Tailwind CSS v4 - (54e50be) - Amadeus Mader
- (**just**) drop combined dev recipe - (7c21356) - Amadeus Mader
- (**just**) remove bacon and use just and cargo-watch - (4b23025) - Amadeus Mader
- (**nix**) drop redundant per-system nixosModules - (e30f372) - Amadeus Mader
- (**pinchflat**) optimize API integration with separate sources endpoint - (26185f9) - Amadeus Mader
- replace leptos with axum, maud, and htmx sse - (d720a60) - Amadeus Mader
- remove pinchflat for hofvarpnir - (a493649) - Amadeus Mader
#### Miscellaneous Chores
- (**cargo**) update reqwest features - (aa0fa43) - Amadeus Mader
- (**cargo**) upgrade deps and use native trust store - (1b6c4e1) - Amadeus Mader
- (**clippy**) allow pedantic lints in fetchers and views - (989cec7) - Amadeus Mader
- (**cog**) add conventional commit config - (f7b725e) - Amadeus Mader
- (**css**) rebuild dashboard css - (08d95b6) - Amadeus Mader
- (**deny**) add cargo-deny config and recipe - (6d7297f) - Amadeus Mader
- (**deps**) upgrade flake - (95fc61f) - Amadeus Mader
- (**deps**) upgrade to rust 1.96.1 - (c476068) - Amadeus Mader
- (**deps**) upgrade flake deps - (6453f63) - Amadeus Mader
- (**deps**) upgrade flake and cargo - (d8161d6) - Amadeus Mader
- (**deps**) upgrade flake - (8d003d5) - Amadeus Mader
- (**deps**) bump rustls-webpki from 0.103.9 to 0.103.13 - (f094c6e) - dependabot[bot]
- (**deps**) bump rand from 0.9.2 to 0.9.4 - (dd903b7) - dependabot[bot]
- (**flake**) add deny, audit, cog, and keep-sorted tools - (d19a90a) - Amadeus Mader
- (**flake**) rust 1.96.0 - (4e57108) - Amadeus Mader
- (**just**) add dev cmd and fmt during pre-commit - (cccfdcd) - Amadeus Mader
- (**rust**) user v1.96.0 - (059fa74) - Amadeus Mader
- upgrade pkgs - (9f26b02) - Amadeus Mader
- upgrade rust version - (b872b8f) - Amadeus Mader
- upgrade flake - (7d21a06) - Amadeus Mader
- apply formatting to dashboard-app - (03b722d) - Amadeus Mader
- fix clippy pedantic warnings and add workspace metadata - (7c3cb7f) - Amadeus Mader
- update pre-commit to auto-fix and add fmt check to pre-push - (7fac387) - Amadeus Mader
- add lefthook for git hooks management - (082b44f) - Amadeus Mader
- cleanup server main.rs and align port defaults - (0963ed9) - Amadeus Mader
- update default ports and improve NixOS module - (d6d67ba) - Amadeus Mader

- - -

Changelog generated by [cocogitto](https://github.com/cocogitto/cocogitto).