# Changelog - SideCar

SideCar is our Node.js sidecar runtime - the Rust crate that downloads
the right Node binary for the host platform and spawns it with the
DNS-isolation knobs Mist needs. This file records what we built in our
voice, version by version. Format adapted from
[Keep a Changelog](https://keepachangelog.com/).

## [v2.1] - Full Workbench Lift (April 2026)

We brought SideCar up alongside the rest of the v2.1 wave - DNS knob
integration with Mist, README/CHANGELOG expansion, and
PascalCase/import-path cleanups.

### Added

- **Comprehensive CHANGELOG release history** (`8b4296c`,
  2026-04-17).
- **Comprehensive README expansion** (`2ad9144`, 2026-04-05) with
  benefit-focused rewrite passes (`822fffb`, `d5c8e42`, 2026-04-04).
- **Crate-level rustdoc** rewritten benefit-first in `Library.rs`
  (`0f03a51`, 2026-04-04).

### Changed

- **`LAND_DNS_SERVER` env var renamed to `Resolve`** for DNS
  configuration (`deefbb5`, 2026-04-28). User-facing knob now
  matches the rest of the project's naming style.
- **`tauri-plugin-shell` updated to 2.3** (`d58c328`, 2026-04-22).
- **Spawn-module function signatures and comments reformatted**
  (`752cc09`, 2026-04-11).
- **Relative file links converted to full GitHub URLs** in the
  documentation (`850c2f1`, 2026-04-16).

### Fixed

- **Mist crate import-path casing** corrected (`ee84a1a`,
  2026-04-18). Followed Mist's `mist` → `Mist` rename.

## [v2.0] - Editor Launch (Q1 2026: SideCar Born)

The pivotal cycle. SideCar was created as a brand-new element on
**2026-03-24** to centralise the Node.js download/spawn logic that
previously lived inline in Mountain.

### Added

- **Initial scaffold** with two binaries (`48f7bdb`, 2026-03-24):
  - **`Download.rs`** - downloads the platform-specific Node binary
    on first run.
  - **`SideCar` binary** (`main.rs` + `Spawn.rs` + `Library.rs`) -
    spawns Node child processes with the DNS override that Mist's
    isolation depends on.
- **Cargo manifest** with the dependency set the sidecar surface
  needs: `tokio`, `reqwest`, `tauri-build`, `tar`, `flate2`, `zip`
  for the platform binary unpacking, plus `serde`, `colored`,
  `anyhow` for the tooling layer.

### Changed

- **Reset SideCar** to remove git LFS tracking and the NODE binaries
  from version control (`48f7bdb`, 2026-03-24). Node binaries are
  now downloaded locally by developers; `.gitignore` excludes the
  NODE content and `.gitattributes` is gone.

## [v0.0] - Project Inception (March 2026)

SideCar didn't exist before March 2026 - the Node.js download/spawn
work used to live inline in Mountain. We extracted it into its own
element so the DNS-isolation handshake with Mist could be a clean,
testable concern instead of a buried branch in Mountain's spawn
path.
