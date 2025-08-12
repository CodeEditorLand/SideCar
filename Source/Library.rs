#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case, non_upper_case_globals)]

#[allow(dead_code)]
/// Main executable function.
fn main() {
	if let Err(Error) = Download::Fn() {
		error!("The application encountered a fatal error: {}", Error);

		std::process::exit(1);
	}
}

pub mod Download;

use log::error;
