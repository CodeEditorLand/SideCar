#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments
)]

//! SideCar binary entry point
//!
//! Application entry point that runs the full sidecar vendoring pipeline
//! (platform resolution, download, cache and `.gitattributes` bookkeeping)
//! directly against [`SideCar::Download`]. Telemetry is initialised exactly
//! once inside [`SideCar::Download::Fn`] - the shared pipeline entry point
//! used by both the `SideCar` and `Download` binaries - so this entry point
//! must not initialise it a second time. Checksum verification is not
//! performed today. The library target intentionally holds no `main`.

use log::error;

fn main() {
	// [Boot] [Telemetry] Telemetry is deliberately NOT initialised here.
	// `SideCar::Download::Fn` (called below) is the single canonical init point
	// for both binaries. Initialising here as well would call Common's
	// non-idempotent initializer twice and emit a duplicate session event.

	// The vendoring pipeline is the binary's responsibility. A fatal error is
	// logged with its cause and reported through the process exit status - the
	// pipeline itself degrades per-file, so this path is reached only when the
	// run cannot produce a usable sidecar at all.
	if let Err(Error) = SideCar::Download::Fn() {
		error!("The application encountered a fatal error: {}", Error);

		std::process::exit(1);
	}
}
