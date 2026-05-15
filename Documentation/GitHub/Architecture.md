# SideCar: Vendored Runtime Manager

This document describes SideCar, the vendored runtime binary manager for Land.
SideCar handles the download, caching, verification, and management of Node.js
runtime binaries for each target platform.

---

## Table of Contents

1. [Overview](#overview)
2. [Architecture](#architecture)
3. [Binary Resolution](#binary-resolution)
4. [Download System](#download-system)
5. [Spawn System](#spawn-system)
6. [Supported Platforms](#supported-platforms)
7. [Caching Strategy](#caching-strategy)
8. [Related Documentation](#related-documentation)

---

## Overview

SideCar is a Rust library and binary that manages pre-compiled native dependency
binaries for Land. It currently handles Node.js runtime binaries, downloading
them from official sources and making them available to Mountain at build time
and runtime.

| Attribute    | Value                                                 |
| ------------ | ----------------------------------------------------- |
| Language     | Rust (edition 2024)                                   |
| Crate type   | Library + Binary                                      |
| Dependencies | tokio, reqwest, serde, zip, tar, flate2, Common, Mist |
| Consumed by  | Mountain (build-time binary selection)                |
| Storage      | SideCar/Cache.json + per-platform cached binaries     |

---

## Architecture

```
+---------------------------------------------------------------+
|                        SideCar                                 |
|                                                                |
|  +----------------------+  +----------------------+            |
|  | Download.rs          |  | Spawn.rs             |            |
|  | - Archive extraction |  | - Node process spawn |            |
|  | - Platform targeting |  | - DNS override       |            |
|  | - Version resolution |  | - Environment setup  |            |
|  +----------------------+  +----------------------+            |
|                                                                |
|  +----------------------+                                      |
|  | Cache.json           |                                      |
|  | - Version-to-path    |                                      |
|  |   mapping            |                                      |
|  | - Checksums          |                                      |
|  +----------------------+                                      |
+---------------------------------------------------------------+
```

### Module Map

| Path                 | Purpose                                                 |
| -------------------- | ------------------------------------------------------- |
| `Source/Download.rs` | Binary download, archive extraction, platform targeting |
| `Source/Spawn.rs`    | Node.js sidecar process spawning with DNS override      |
| `Source/Library.rs`  | Library root                                            |
| `Source/main.rs`     | Binary entry point for standalone operation             |

---

## Binary Resolution

The resolution process selects the correct Node.js binary for the target
platform:

```
Build.sh reads NodeVersion and NodePlatform from .env.Land
    |
    v
SideCar::resolve()
    |
    +---> Check SideCar/Cache.json for cached binary
    |       |
    |       +---> CACHED: Return cached path
    |       |
    |       +---> NOT CACHED: Continue
    |
    +---> Determine target platform string
    |       - darwin-arm64 (Apple Silicon)
    |       - darwin-x64 (Intel)
    |       - linux-arm64
    |       - linux-x64
    |
    +---> Construct download URL:
    |       https://nodejs.org/dist/v{version}/node-v{version}-{platform}.tar.gz
    |
    +---> Download binary (via Download.rs)
    +---> Verify SHA-256 checksum
    +---> Extract to SideCar/{platform}/node
    +---> Record in Cache.json
    |
    v
Return resolved binary path to Mountain
```

### Version Resolution Priority

1. `NodeVersion` environment variable (explicit version override)
2. `SideCar/Cache.json` latest cached version
3. Node.js LTS version (fallback default)

---

## Download System

The `Download` module handles archive retrieval and extraction:

| Operation            | Description                                                   |
| -------------------- | ------------------------------------------------------------- |
| URL construction     | Builds platform-specific download URL from version + platform |
| HTTP download        | Streaming download with timeout and retry                     |
| SHA-256 verification | Checksum verification against published hash                  |
| Archive extraction   | `.tar.gz` decompression with platform prefix stripping        |
| Binary placement     | Single binary extracted to `SideCar/{platform}/node`          |

### Download Flow

```
1. HTTP GET to https://nodejs.org/dist/v{version}/SHASUMS256.txt
2. Parse SHASUMS256.txt for target binary hash
3. HTTP GET to https://nodejs.org/dist/v{version}/node-v{version}-{platform}.tar.gz
4. Stream to temporary download file with progress tracking
5. SHA-256 hash computed during download
6. Verify hash against published checksum
7. Extract node binary from archive
8. Move binary to SideCar/{platform}/node
9. Update Cache.json
```

---

## Spawn System

The `Spawn` module manages starting Node.js sidecar processes with the correct
binary and environment:

| Feature            | Description                                                   |
| ------------------ | ------------------------------------------------------------- |
| Binary selection   | Uses cached Node.js binary for target platform                |
| DNS override       | Configures process to use Mist's local DNS resolver           |
| Environment setup  | Sets PATH, NODE_PATH, and Land-specific environment variables |
| Process monitoring | Watches for process exit and captures output                  |

### Spawn Configuration

```rust
let node_path = SideCar::resolve("22.0.0", "darwin-arm64")?;
let child = SideCar::spawn(
    &node_path,
    &["bootstrap-fork.js"],
    SpawnConfig {
        env: vec![
            ("VINE_PORT", "50051"),
            ("MIST_PORT", "5380"),
            ("NODE_PATH", "/usr/local/lib/node_modules"),
        ],
        dns_override: Some("127.0.0.1:5380"),
        cwd: Some("/Applications/Land.app/Contents/Resources"),
    },
)?;
```

---

## Supported Platforms

| Target Triple               | Platform String | Archive Pattern                       |
| --------------------------- | --------------- | ------------------------------------- |
| `aarch64-apple-darwin`      | `darwin-arm64`  | `node-v{version}-darwin-arm64.tar.gz` |
| `x86_64-apple-darwin`       | `darwin-x64`    | `node-v{version}-darwin-x64.tar.gz`   |
| `aarch64-unknown-linux-gnu` | `linux-arm64`   | `node-v{version}-linux-arm64.tar.gz`  |
| `x86_64-unknown-linux-gnu`  | `linux-x64`     | `node-v{version}-linux-x64.tar.gz`    |

---

## Caching Strategy

SideCar maintains a JSON-based cache manifest at `SideCar/Cache.json`:

```json
{
	"version": "1",
	"entries": {
		"22.0.0-darwin-arm64": {
			"path": "aarch64-apple-darwin/node",
			"sha256": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
			"downloaded_at": "2026-01-15T10:30:00Z",
			"size": 68700000
		}
	}
}
```

| Cache Key       | Value                  | Description                         |
| --------------- | ---------------------- | ----------------------------------- |
| Version key     | `{version}-{platform}` | Uniquely identifies a binary        |
| `path`          | Relative path          | Location of cached binary           |
| `sha256`        | Hex string             | Checksum for integrity verification |
| `downloaded_at` | ISO 8601               | Timestamp of download               |
| `size`          | Bytes                  | Binary file size                    |

Cache entries are invalidated when:

- A new version is requested that differs from the cached version
- SHA-256 verification fails on the cached binary
- The cache file is manually cleared

---

## Related Documentation

- [Mountain](../Mountain/Documentation/GitHub/Architecture.md) - Main backend
  (binary consumer)
- [Mist](../Mist/Documentation/GitHub/Architecture.md) - DNS isolation for
  spawned processes
- [BuildPipeline](../../../Documentation/GitHub/BuildPipeline.md) - Build
  pipeline integration
- [RustInfrastructure](../../../Documentation/GitHub/RustInfrastructure.md) -
  Rust backend components

---

**Project Maintainers:** Source Open
([Source/Open@Editor.Land](mailto:Source/Open@Editor.Land)) |
[GitHub Repository](https://github.com/CodeEditorLand/SideCar) |
[Report an Issue](https://github.com/CodeEditorLand/SideCar/issues)
