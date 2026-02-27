#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]

/// Main executable function.
/// TODO: Move this function to main.rs in a future refactor
#[allow(dead_code)]
pub fn main() {
	if let Err(Error) = Download::Fn() {
		error!("The application encountered a fatal error: {}", Error);

		std::process::exit(1);
	}
}

pub mod Download;
pub mod Spawn;

use log::error;
