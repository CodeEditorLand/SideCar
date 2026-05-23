//! SideCar binary entry point
//!
//! This file serves as the main entry point for the SideCar application.
//! It calls the main function from the library.

fn main() {
	// [Boot] [Telemetry] Bring up shared dual-pipe (PostHog + OTLP) on a
	// short-lived tokio runtime - SideCar's main is sync, but Common's
	// Initialize is async because the posthog-rs constructor is. No-op
	// in release builds and when `Capture=false`.
	if let Ok(Runtime) = tokio::runtime::Runtime::new() {
		Runtime.block_on(CommonLibrary::Telemetry::Initialize::Fn(
			CommonLibrary::Telemetry::Tier::Tier::SideCar,
		));
	}

	// DEPENDENCY: Move the main function from Library.rs here in a future refactor
	// Currently Library.rs contains both lib and binary code
	// For now, delegate to the library's main function
	use SideCar::main as lib_main;

	lib_main();
}
