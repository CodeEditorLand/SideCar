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
//! Application entry point that initialises telemetry, then runs the full
//! sidecar vendoring pipeline (platform resolution, download, checksum
//! verification, cache and `.gitattributes` bookkeeping) directly against
//! [`SideCar::Download`]. The library target intentionally holds no `main`.

use log::error;

fn main() {
	// [Boot] [Telemetry] Bring up shared dual-pipe (PostHog + OTLP) on a
	// short-lived tokio runtime - SideCar's main is sync, but Common's
	// Initialize is async because the posthog-rs constructor is. No-op
	// in release builds and when `Capture=false`. A runtime that cannot be
	// built is a non-happy path: skip telemetry and keep vendoring running.
	if let Ok(Runtime) = tokio::runtime::Runtime::new() {
		Runtime.block_on(CommonLibrary::Telemetry::Initialize::Fn(
			CommonLibrary::Telemetry::Tier::Tier::SideCar,
		));
	} else {
		error!("[Boot] [Telemetry] Could not build a tokio runtime; vendoring continues without telemetry");
	}

	// The vendoring pipeline is the binary's responsibility. A fatal error is
	// logged with its cause and reported through the process exit status - the
	// pipeline itself degrades per-file, so this path is reached only when the
	// run cannot produce a usable sidecar at all.
	if let Err(Error) = SideCar::Download::Fn() {
		error!("The application encountered a fatal error: {}", Error);

		std::process::exit(1);
	}
}
