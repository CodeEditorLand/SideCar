#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments
)]

//! # SideCar: Pre-Built Node.js Binary Manager
//!
//! Cocoon needs Node.js to run VS Code extensions. SideCar manages the
//! embedded Node.js binary: downloading platform-specific builds, verifying
//! integrity, and spawning Node.js as a Tauri sidecar process.
//!
//! No system Node.js installation required - Land ships its own.
//!
//! ## Overview
//!
//! 1. **Downloads** the correct Node.js binary for the current OS and arch
//! 2. **Verifies** the download checksum before extracting
//! 3. **Spawns** Node.js as a managed sidecar that Mountain can monitor
//!
//! ## Modules
//!
//! - [`Download`]: Platform-aware binary fetching and checksum verification
//!
//! ## Binary targets
//!
//! - `SideCar` (`Source/main.rs`): Application entry point that initialises
//!   telemetry and delegates to [`main`].
//! - `Download` (`Source/Download.rs`): Standalone vendoring binary that
//!   downloads sidecar runtimes into the repository.

/// Main executable function.
///
/// Initialises telemetry via [`CommonLibrary::Telemetry`], sets up the logger,
/// then runs the full sidecar vendoring pipeline: resolving the platform
/// matrix, fetching sidecar definitions, downloading and verifying each binary,
/// updating the cache, and managing `.gitattributes` for Git LFS tracking.
///
/// ## Panics
///
/// Exits the process with status 1 if the vendoring pipeline encounters a
/// fatal error.
///
/// DEPENDENCY: Move this function to main.rs in a future refactor
#[allow(dead_code)]
pub fn main() {
	if let Err(Error) = Download::Fn() {
		error!("The application encountered a fatal error: {}", Error);

		std::process::exit(1);
	}
}

/// Platform-aware binary download and verification module.
///
/// Downloads Node.js binaries for the current OS/arch, verifies checksums,
/// manages a download cache (`Cache.json`), and maintains `.gitattributes`
/// for Git LFS tracking.
pub mod Download;

use log::error;
