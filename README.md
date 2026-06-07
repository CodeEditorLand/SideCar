<table>
  <tr>
    <td align="left" valign="middle">
      <h3 align="left">
        SideCar&#x2001;⚙️
      </h3>
    </td>
    <td align="left" valign="middle">
      <h3 align="left">
        +
      </h3>
    </td>
    <td align="left" valign="middle">
      <h3 align="left">
        <a href="https://Land.PlayForm.Cloud" target="_blank">
          <picture>
            <source media="(prefers-color-scheme: dark)" srcset="https://PlayForm.Cloud/Dark/Image/GitHub/Land.svg" />
            <source media="(prefers-color-scheme: light)" srcset="https://PlayForm.Cloud/Image/GitHub/Land.svg" />
            <img width="28" alt="Land Logo" src="https://PlayForm.Cloud/Image/GitHub/Land.svg" />
          </picture>
        </a>
      </h3>
    </td>
    <td align="left" valign="middle">
      <h3 align="left">
        <a href="https://Land.PlayForm.Cloud" target="_blank">
          Land&#x2001;🏞️
        </a>
      </h3>
    </td>
    </td>
  </tr>
</table>

---

# **SideCar**&#x2001;⚙️

Pre-Compiled Native Dependencies for Land &#x2001;🏞️

> **VS Code ships one Node.js binary and detects the platform at runtime, with
> fallback chains that fail in edge cases (Alpine Linux, custom glibc versions,
> ARM configurations).**

_"The right Node.js binary. Compiled in. No detection."_

SideCar packages the exact Node.js binary for each target triple at compile
time: `aarch64-apple-darwin`, `x86_64-pc-windows-msvc`, and four others. Cocoon
always gets the binary that matches the host. No runtime detection, no fallback
chains, no surprises.

Welcome to **SideCar**, the central repository for all pre-compiled,
platform-specific sidecar binaries required by the **Land Code Editor**
ecosystem. A "sidecar" is an external, standalone executable that runs alongside
the main `Mountain` application to provide specialized functionality, such as
the `Cocoon` extension host which runs on Node.js.

**SideCar** is engineered to:

1. **Provide Portable Runtimes**: Vendored Node.js and other runtimes eliminate
   user dependency requirements.
2. **Enable Deterministic Builds**: Organized by target triple for build-time
   binary selection.
3. **Support Multiple Platforms**: Comprehensive matrix for macOS, Linux, and
   Windows on x86_64 and aarch64 architectures.
4. **Automate Download Management**: Automated fetching, caching, and Git LFS
   management of runtime binaries.

---

## Directory Structure&#x2001;📁

```
SideCar/
├── Source/
│   ├── Download.rs              # Main download binary: fetches, verifies, and organizes platform binaries.
│   ├── Library.rs               # Module declarations and shared utilities.
│   └── main.rs                  # Binary entry point for the download tool.
├── build.rs                     # Build script: binary selection and staging for the final installer.
├── Cargo.toml
├── Cache.json                   # Download cache metadata (tracks fetched versions per platform).
├── aarch64-apple-darwin/        # macOS Apple Silicon binaries (Node.js per version).
├── x86_64-apple-darwin/         # macOS Intel binaries.
├── x86_64-pc-windows-msvc/      # Windows x64 binaries.
├── aarch64-unknown-linux-gnu/   # Linux ARM64 (glibc) binaries.
├── x86_64-unknown-linux-gnu/    # Linux x64 (glibc) binaries.
└── Resource/                    # Shared resources bundled with sidecars.
```

### Supported Target Triples

| Target Triple               | Platform            |
| :-------------------------- | :------------------ |
| `aarch64-apple-darwin`      | macOS Apple Silicon |
| `x86_64-apple-darwin`       | macOS Intel         |
| `x86_64-pc-windows-msvc`    | Windows x64         |
| `aarch64-pc-windows-msvc`   | Windows ARM64       |
| `x86_64-unknown-linux-gnu`  | Linux x64 (glibc)   |
| `aarch64-unknown-linux-gnu` | Linux ARM64 (glibc) |

### How It's Used

The
[`Download`](https://github.com/CodeEditorLand/SideCar/tree/Current/Source/Download.rs)
Rust binary is responsible for populating this structure. It fetches the
official distributions for various sidecars and platforms and organizes them
according to the convention above.

During the application build, the main `Build.rs` orchestrator uses this
repository as a source. Based on build flags (e.g., `--node-version=22`), it
selects the appropriate executable from this directory and prepares it for
bundling into the final application installer.

---

## Key Features&#x2001;🔐

- **Concurrent Downloads**: Parallel downloading of multiple runtime binaries
  using Tokio for maximum throughput.
- **Intelligent Caching**: Maintains a `Cache.json` file to track downloaded
  versions and avoid redundant downloads.
- **Version Resolution**: Automatically resolves major versions to latest patch
  from nodejs.org and other sources.
- **Git LFS Management**: Automatic `.gitattributes` updates for large binary
  tracking in Git LFS.
- **Platform Matrix**: Comprehensive support for x86_64 and aarch64
  architectures across macOS, Linux, and Windows.

---

## Core Architecture Principles&#x2001;🏗️

| Principle                   | Description                                                                          | Key Components Involved                       |
| :-------------------------- | :----------------------------------------------------------------------------------- | :-------------------------------------------- |
| **Deterministic Selection** | Organize binaries by target triple for deterministic build-time selection.           | Directory structure, target triple convention |
| **Version Tracking**        | Maintain cache metadata to avoid redundant downloads and ensure version consistency. | `Cache.json`, version resolution              |
| **Git LFS Integration**     | Automatically manage Git LFS pointers for large binary tracking.                     | `.gitattributes` management                   |

---

## `SideCar` in the Land Ecosystem&#x2001;⚙️ + &#x2001;🏞️

| Component         | Role & Key Responsibilities                                           |
| :---------------- | :-------------------------------------------------------------------- |
| **Download Tool** | Populates the SideCar directory with pre-compiled runtime binaries.   |
| **Cache Manager** | Tracks downloaded versions in `Cache.json` for build reproducibility. |
| **Build Source**  | Provides vendored runtimes to `Mountain` during the build process.    |

---

## System Architecture Diagram&#x2001;🏗️

This diagram illustrates how `SideCar` vendors and organizes runtime
dependencies.

```mermaid
graph LR
    classDef sidecar  fill:#ffe0cc,stroke:#e67e22,stroke-width:2px,color:#4a1500;
    classDef external fill:#ebebeb,stroke:#888,stroke-width:1px,stroke-dasharray:5 5,color:#333;
    classDef storage  fill:#cce8ff,stroke:#2980b9,stroke-width:1px,color:#003050;
    classDef mountain fill:#f0d0ff,stroke:#9b59b6,stroke-width:1px,color:#2c0050;
    classDef cocoon   fill:#d0d8ff,stroke:#4a6fa5,stroke-width:1px,color:#001050;

    subgraph SOURCES["External Sources"]
        NodeJSOrg["nodejs.org\n(official distributions)"]:::external
    end

    subgraph SIDECAR["SideCar ⚙️ - Vendored Runtime Manager"]
        direction TB
        subgraph TOOL["Source/ - Download Tool (Rust binary)"]
            DownloadBin["Download.rs\nfetch · verify · organise\nTokio parallel downloads"]:::sidecar
            CacheJSON["Cache.json\nversion tracking\navoid redundant downloads"]:::sidecar
            GitLFS[".gitattributes\nGit LFS pointers\nfor large binaries"]:::sidecar
            DownloadBin --> CacheJSON
            DownloadBin --> GitLFS
        end
        subgraph LAYOUT["Directory Layout (by target triple)"]
            Darwin_ARM["aarch64-apple-darwin/\nNode.js Apple Silicon"]:::storage
            Darwin_x86["x86_64-apple-darwin/\nNode.js macOS Intel"]:::storage
            Win_x86["x86_64-pc-windows-msvc/\nNode.js Windows x64"]:::storage
            Linux_ARM["aarch64-unknown-linux-gnu/"]:::storage
            Linux_x86["x86_64-unknown-linux-gnu/"]:::storage
        end
        BuildRS["build.rs\nbinary selection at build time\nstages correct triple into installer"]:::sidecar

        DownloadBin --> LAYOUT
        BuildRS --> LAYOUT
    end

    subgraph CONSUMERS["Consumers at Runtime"]
        Mountain["Mountain ⛰️\nbuild.rs bundles chosen binary\ninto Tauri installer"]:::mountain
        Cocoon["Cocoon 🦋\ngets correct Node.js binary\nno runtime detection needed"]:::cocoon
    end

    NodeJSOrg --> DownloadBin
    LAYOUT --> BuildRS
    BuildRS --> Mountain
    Mountain -- spawns with Spawn.rs --> Cocoon
```

---

## Deep Dive & Component Breakdown&#x2001;🔬

To understand how `SideCar`'s download tool works, see the following source
files:

- **[`Source/Download.rs`](https://github.com/CodeEditorLand/SideCar/tree/Current/Source/Download.rs)** -
  Main download binary entry point
- **[`Cache.json`](https://github.com/CodeEditorLand/SideCar/tree/Current/Cache.json)** -
  Download cache tracking file
- **[`.gitattributes`](https://github.com/CodeEditorLand/SideCar/tree/Current/.gitattributes)** -
  Git LFS configuration for large binaries

The download tool handles concurrent downloads via Tokio, version resolution
from nodejs.org, and automatic Git LFS management for tracking large binary
files.

---

## Getting Started&#x2001;🚀

### Running the Download Tool

```sh
# Build the download tool
cd Element/SideCar
cargo build --release

# Run to download and organize all sidecars
./Target/release/Download
```

**Key Dependencies:**

- `tokio`: Async runtime for concurrent downloads
- `reqwest`: HTTP client for fetching binaries
- `serde`/`serde_json`: Cache.json serialization
- `git2`: Git LFS management

### Usage Pattern&#x2001;🚀

The SideCar directory is populated once during project setup:

1. **Build Download Tool:** Compile the `Download` binary
2. **Run Download:** Execute to fetch and organize all runtime binaries
3. **Build Mountain:** The build system selects appropriate binaries from
   SideCar

> [!NOTE]
>
> The contents of this directory are generated by the
> [`Download`](https://github.com/CodeEditorLand/SideCar/tree/Current/Source/Download.rs)
> Rust binary and consist of large, third-party binaries. This directory
> **should not be committed to version control** and should be added to the
> project's `.gitignore` file. The tool should be run once to vendor the
> dependencies as part of the initial project setup.

---

**Parent Project**:
[`Mountain`](https://github.com/CodeEditorLand/Mountain/tree/Current/README.md)
| **Related Directory**:
[`Binary`](https://github.com/CodeEditorLand/Mountain/tree/Current/Binary/README.md)

---

## See Also

- [SideCar Documentation](https://land.playform.cloud/Doc/sidecar)
- [Architecture Overview](https://land.playform.cloud/Doc/architecture)
- [Why Rust](https://land.playform.cloud/Doc/why-rust)
- [Cocoon](https://github.com/CodeEditorLand/Cocoon)
- [Mountain](https://github.com/CodeEditorLand/Mountain)

---

## License&#x2001;⚖️

This project is released into the public domain under the **Creative Commons CC0
Universal** license. You are free to use, modify, distribute, and build upon
this work for any purpose, without any restrictions. For the full legal text,
see the [`LICENSE`](https://github.com/CodeEditorLand/SideCar/tree/Current/)
file.

---

## Changelog&#x2001;📜

Stay updated with our progress! See
[`CHANGELOG.md`](https://github.com/CodeEditorLand/SideCar/tree/Current/) for a
history of changes specific to **SideCar**.

---

## Funding & Acknowledgements&#x2001;🙏🏻

**SideCar** is a core element of the **Land** ecosystem. This project is funded
through [NGI0 Commons Fund](https://NLnet.NL/commonsfund), a fund established by
[NLnet](https://NLnet.NL) with financial support from the European Commission's
[Next Generation Internet](https://ngi.eu) program. Learn more at the
[NLnet project page](https://NLnet.NL/project/Land).

The project is operated by PlayForm, based in Sofia, Bulgaria.

PlayForm acts as the open-source steward for Code Editor Land under the NGI0
Commons Fund grant.

<table>
  <thead>
    <tr>
      <th align="left">
        <strong>
          Land
        </strong>
      </th>
      <th align="left">
        <strong>
          PlayForm
        </strong>
      </th>
      <th align="left">
        <strong>
          NLnet
        </strong>
      </th>
      <th align="left">
        <strong>
          NGI0 Commons Fund
        </strong>
      </th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td align="left" valign="middle">
        <a href="https://Land.PlayForm.Cloud">
          <img width="60" src="https://raw.githubusercontent.com/CodeEditorLand/Asset/refs/heads/Current/Logo/Land.svg" alt="Land" />
        </a>
      </td>
      <td align="left" valign="middle">
        <a href="https://PlayForm.Cloud">
          <img width="76" src="https://raw.githubusercontent.com/PlayForm/Asset/refs/heads/Current/Logo/PlayForm.svg" alt="PlayForm" />
        </a>
      </td>
      <td align="left" valign="middle">
        <a href="https://NLnet.NL">
          <img width="240" src="https://NLnet.NL/logo/banner.svg" alt="NLnet" />
        </a>
      </td>
      <td align="left" valign="middle">
        <a href="https://NLnet.NL/commonsfund">
          <img width="240" src="https://NLnet.NL/image/logos/NGI0CommonsFund_tag_black_mono.svg" alt="NGI0 Commons Fund" />
        </a>
      </td>
    </tr>
  </tbody>
</table>

---

**Project Maintainers**: Source Open
([Source/Open@Land.PlayForm.Cloud](mailto:Source/Open@Land.PlayForm.Cloud)) |
[GitHub Repository](https://github.com/CodeEditorLand/SideCar) |
[Report an Issue](https://github.com/CodeEditorLand/SideCar/issues) |
[Security Policy](https://github.com/CodeEditorLand/SideCar/security/policy)
