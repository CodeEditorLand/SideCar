# Changelog

All notable changes to the SideCar element are documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com/).

SideCar is the Node.js binary distribution layer — it packages and bundles
the Node.js runtime binary for each target platform (macOS aarch64, Linux
x86_64/aarch64/armv7, Windows x64) for Tauri sidecar consumption by Cocoon.

## [v2.1] — Q2 2026: Documentation Links

### Changed

- Converted relative file links to full GitHub URLs in all documentation
- Cache.json version tracking for binary artifact checksums
- Documentation cross-references updated

## [v2.0] — Q1 2026: Project Creation

### Added

- Repository created March 24, 2026 to consolidate Node.js sidecar
  distribution logic previously scattered across Mountain and Maintain
- Platform-specific binary directories:
  - `aarch64-apple-darwin/` — Apple Silicon macOS
  - `aarch64-unknown-linux-gnu/` — ARM64 Linux
  - Additional targets for x86_64 Linux/Windows
- 19 Rust source files implementing:
  - Binary fetching and verification
  - Platform detection and selection
  - Checksum validation (`Cache.json`)
  - Tauri sidecar integration helpers
- `build.rs` for cross-platform build configuration
- Cargo workspace integration as `SideCar` member of Land root workspace
- PascalCase naming convention throughout
- CI/CD workflows for automated binary updates
