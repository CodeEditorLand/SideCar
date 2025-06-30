// ==============================================================================
// Universal Sidecar Vendor - Rust Edition
//
// This program automates downloading and organizing full distributions of
// various sidecar runtimes (like Node.js) for a Tauri application. It is a Rust
// rewrite of the original shell script, enhanced with modern features.
//
// Key Features:
//   - Asynchronous, Concurrent Downloads: Leverages Tokio to download multiple
//     binaries in parallel, significantly speeding up the process.
//   - Intelligent Caching: Maintains a `Cache.json` file to track downloaded
//     versions. It automatically detects if a newer patch version is available
//     for a requested major version and updates the binary.
//   - Extensible Design: Easily configured to support new sidecars, versions,
//     and platforms.
//   - Robust Error Handling: Uses `anyhow` for clear and concise error
//     reporting.
//   - Preserved File Structure: The final output directory structure remains
//     identical to the original script (`Architecture/SidecarName/Version`).
//
// ==============================================================================

// Allow non_snake_case to meet the user's naming convention requirement.
#![allow(non_snake_case, non_upper_case_globals)]

// --- Type Definitions and Structs ---

/// Represents a single platform target for which binaries will be downloaded.
/// This struct holds all the necessary identifiers for a given platform.
#[derive(Clone, Debug)]
struct PlatformTarget {
	/// The identifier used in the download URL (e.g., "win-x64",
	///
	/// "linux-arm64").
	DownloadIdentifier:String,

	/// The file extension of the archive (e.g., "zip", "tar.gz").
	ArchiveExtension:String,

	/// The official Tauri target triple for this platform (e.g.,
	///
	/// "x86_64-pc-windows-msvc").
	TauriTargetTriple:String,
}

/// Defines the type of archive being handled, which determines the extraction
/// logic.
#[derive(Clone, Debug, PartialEq)]
enum ArchiveType {
	Zip,

	TarGz,
}

/// Represents a specific version of Node.js as returned by the official index.
/// Used for deserializing the JSON response from `nodejs.org`.
#[derive(Deserialize, Debug)]
struct NodeVersionInfo {
	version:String,
}

/// Contains all the necessary information to perform a single download and
/// installation task. An instance of this struct is created for each binary
/// that needs to be downloaded.
#[derive(Clone, Debug)]
struct DownloadTask {
	/// The name of the sidecar (e.g., "NODE").
	SidecarName:String,

	/// The major version string requested (e.g., "24").
	MajorVersion:String,

	/// The full, resolved version string (e.g., "v24.0.0").
	FullVersion:String,

	/// The complete URL to download the archive from.
	DownloadURL:String,

	/// The final destination directory for the extracted binaries.
	DestinationDirectory:PathBuf,

	/// The type of archive to be downloaded.
	ArchiveType:ArchiveType,

	/// The name of the root folder inside the archive once extracted.
	ExtractedFolderName:String,

	/// The Tauri target triple for this download task.
	TauriTargetTriple:String,
}

/// Represents the structure of the `Cache.json` file.
/// It uses a HashMap to map a unique key (representing a specific
/// sidecar/version/platform) to the full version string that was last
/// downloaded.
#[derive(Serialize, Deserialize, Debug, Default)]
struct DownloadCache {
	/// The core data structure for the cache.
	/// Key: A unique string like "x86_64-pc-windows-msvc/NODE/24".
	/// Value: The full version string, like "v24.0.0".
	Entries:HashMap<String, String>,
}

impl DownloadCache {
	/// Loads the cache from the `Cache.json` file in the base sidecar
	/// directory. If the file doesn't exist, it returns a new, empty cache.
	fn Load(CachePath:&Path) -> Self {
		if !CachePath.exists() {
			info!("Cache file not found. A new one will be created.");

			return DownloadCache::default();
		}

		let FileContents = match fs::read_to_string(CachePath) {
			Ok(Contents) => Contents,

			Err(Error) => {
				warn!("Failed to read cache file: {}. Starting with an empty cache.", Error);

				return DownloadCache::default();
			},
		};

		match serde_json::from_str(&FileContents) {
			Ok(Cache) => {
				info!("Successfully loaded download cache.");

				Cache
			},

			Err(Error) => {
				warn!("Failed to parse cache file: {}. Starting with an empty cache.", Error);

				DownloadCache::default()
			},
		}
	}

	/// Saves the current state of the cache to the `Cache.json` file.
	/// The JSON is pretty-printed for readability.
	fn Save(&self, CachePath:&Path) -> Result<()> {
		let File =
			File::create(CachePath).with_context(|| format!("Failed to create cache file at {:?}", CachePath))?;

		serde_json::to_writer_pretty(File, self).with_context(|| "Failed to serialize and write to cache file.")?;

		Ok(())
	}
}

// --- Configuration ---

/// Returns the root directory where all sidecars will be stored.
/// This is determined dynamically by navigating up from the executable's
/// location. It assumes the executable is located in a path like
/// `.../SideCar/Target/release/`, and it will resolve the base path to
/// `.../SideCar/`.
fn GetBaseSidecarDirectory() -> Result<PathBuf> {
	// Get the full path to the currently running executable.
	let CurrentExePath = env::current_exe().context("Failed to get the path of the current executable.")?;

	// The first .parent() gets the directory containing the exe (e.g., `release`).
	// We then navigate up two more levels to get to the intended `SideCar`
	// directory.
	let BaseDirectory = CurrentExePath
        .parent() // -> .../SideCar/Target/release
        .and_then(|p| p.parent()) // -> .../SideCar/Target
        .and_then(|p| p.parent()) // -> .../SideCar
        .context("Could not determine the base sidecar directory. Expected to be run from a subdirectory like 'Target/release' within the sidecar project.")?;

	Ok(BaseDirectory.to_path_buf())
}

/// Defines the matrix of platforms to target. Each entry specifies how to
/// download and identify binaries for a specific architecture.
fn GetPlatformMatrix() -> Vec<PlatformTarget> {
	vec![
		PlatformTarget {
			DownloadIdentifier:"win-x64".to_string(),

			ArchiveExtension:"zip".to_string(),

			TauriTargetTriple:"x86_64-pc-windows-msvc".to_string(),
		},
		PlatformTarget {
			DownloadIdentifier:"linux-x64".to_string(),

			ArchiveExtension:"tar.gz".to_string(),

			TauriTargetTriple:"x86_64-unknown-linux-gnu".to_string(),
		},
		PlatformTarget {
			DownloadIdentifier:"linux-arm64".to_string(),

			ArchiveExtension:"tar.gz".to_string(),

			TauriTargetTriple:"aarch64-unknown-linux-gnu".to_string(),
		},
		PlatformTarget {
			DownloadIdentifier:"darwin-x64".to_string(),

			ArchiveExtension:"tar.gz".to_string(),

			TauriTargetTriple:"x86_64-apple-darwin".to_string(),
		},
		PlatformTarget {
			DownloadIdentifier:"darwin-arm64".to_string(),

			ArchiveExtension:"tar.gz".to_string(),

			TauriTargetTriple:"aarch64-apple-darwin".to_string(),
		},
	]
}

/// Defines which sidecars and versions to fetch. This structure makes it
/// easy to add more sidecars like Deno in the future.
fn GetSidecarsToFetch() -> HashMap<String, Vec<String>> {
	let mut Sidecars = HashMap::new();

	Sidecars.insert(
		"NODE".to_string(),
		vec!["24", "23", "22", "21", "20", "19", "18", "17", "16"]
			.into_iter()
			.map(String::from)
			.collect(),
	);

	Sidecars
}

// --- Helper Functions ---

/// Environment variable for setting the log level.
pub const LogEnv:&str = "RUST_LOG";

// --- Core Logic ---

/// Fetches the official Node.js versions index from nodejs.org.
async fn FetchNodeVersions(Client:&Client) -> Result<Vec<NodeVersionInfo>> {
	info!("Fetching Node.js version index for resolving versions...");

	let Response = Client
		.get("https://nodejs.org/dist/index.json")
		.send()
		.await
		.context("Failed to send request to Node.js version index.")?;

	if !Response.status().is_success() {
		return Err(anyhow!("Received non-success status from Node.js index: {}", Response.status()));
	}

	let Versions = Response
		.json::<Vec<NodeVersionInfo>>()
		.await
		.context("Failed to parse Node.js version index JSON.")?;

	Ok(Versions)
}

/// Resolves a major version string (e.g., "22") to the latest full patch
/// version (e.g., "v22.3.0") using the fetched version index.
fn ResolveLatestPatchVersion(MajorVersion:&str, AllVersions:&[NodeVersionInfo]) -> Option<String> {
	let VersionPrefix = format!("v{}.", MajorVersion);

	AllVersions
		.iter()
		.find(|v| v.version.starts_with(&VersionPrefix))
		.map(|v| v.version.clone())
}

/// Downloads a file from a URL to a specified path.
async fn DownloadFile(Client:&Client, URL:&str, DestinationPath:&Path) -> Result<()> {
	let mut Response = Client.get(URL).send().await?.error_for_status()?;

	let mut DestinationFile =
		File::create(DestinationPath).with_context(|| format!("Failed to create file at {:?}", DestinationPath))?;

	// Stream the download to handle large files without high memory usage.
	while let Some(Chunk) = Response.chunk().await? {
		DestinationFile.write_all(&Chunk)?;
	}

	Ok(())
}

/// Extracts the contents of a downloaded archive to a target directory.
/// This function handles both `.zip` and `.tar.gz` files selectively,
///
/// mimicking the behavior of the original script.
fn ExtractArchive(
	ArchiveType:&ArchiveType,

	ArchivePath:&Path,

	ExtractionDirectory:&Path,

	ExtractedFolderName:&str,
) -> Result<()> {
	match ArchiveType {
		ArchiveType::Zip => {
			let File = File::open(ArchivePath)?;

			let mut Archive = zip::ZipArchive::new(File)?;

			// Selectively extract required files for Windows
			for i in 0..Archive.len() {
				let mut ZipFile = Archive.by_index(i)?;

				let Output = match ZipFile.enclosed_name() {
					Some(path) => path.to_owned(),

					None => continue,
				};

				// Only extract specific files/folders
				if Output.starts_with(format!("{}/node.exe", ExtractedFolderName))
					|| Output.starts_with(format!("{}/npm", ExtractedFolderName))
					|| Output.starts_with(format!("{}/npx", ExtractedFolderName))
					|| Output.starts_with(format!("{}/corepack", ExtractedFolderName))
					|| Output.starts_with(format!("{}/node_modules/npm/", ExtractedFolderName))
					|| Output.starts_with(format!("{}/node_modules/corepack/", ExtractedFolderName))
				{
					let FullOutput = ExtractionDirectory.join(Output);

					if ZipFile.name().ends_with('/') {
						fs::create_dir_all(&FullOutput)?;
					} else {
						if let Some(p) = FullOutput.parent() {
							if !p.exists() {
								fs::create_dir_all(p)?;
							}
						}
						let mut OutFile = File::create(&FullOutput)?;

						io::copy(&mut ZipFile, &mut OutFile)?;
					}
				}
			}
		},

		ArchiveType::TarGz => {
			let File = File::open(ArchivePath)?;

			let Decompressor = flate2::read::GzDecoder::new(File);

			let mut Archive = tar::Archive::new(Decompressor);

			// Let the `tar` crate handle extraction, but we must filter entries.
			// This is more complex than a full unpack.
			for EntryResult in Archive.entries()? {
				let mut Entry = EntryResult?;

				let Path = Entry.path()?.to_path_buf();

				// Only extract specific files/folders for Unix-like systems
				if Path.starts_with(format!("{}/bin/node", ExtractedFolderName))
					|| Path.starts_with(format!("{}/bin/npm", ExtractedFolderName))
					|| Path.starts_with(format!("{}/bin/npx", ExtractedFolderName))
					|| Path.starts_with(format!("{}/bin/corepack", ExtractedFolderName))
					|| Path.starts_with(format!("{}/lib/node_modules/npm", ExtractedFolderName))
					|| Path.starts_with(format!("{}/lib/node_modules/corepack", ExtractedFolderName))
				{
					Entry.unpack_in(ExtractionDirectory)?;
				}
			}
		},
	}
	Ok(())
}

/// The main asynchronous function for processing a single download task.
/// This function is designed to be run concurrently for multiple tasks.
async fn ProcessDownloadTask(Task:DownloadTask, Client:Client, Cache:Arc<Mutex<DownloadCache>>) -> Result<()> {
	let TempDirectory = Builder::new().prefix("sidecar-download-").tempdir()?;

	let ArchiveName = Task.DownloadURL.split('/').last().unwrap_or("download.tmp");

	let ArchivePath = TempDirectory.path().join(ArchiveName);

	info!(
		"      [{}/{}] Downloading from: {}",
		Task.TauriTargetTriple, Task.SidecarName, Task.DownloadURL
	);

	if let Err(Error) = DownloadFile(&Client, &Task.DownloadURL, &ArchivePath).await {
		error!(
			"      [{}/{}] Failed to download {}: {}",
			Task.TauriTargetTriple, Task.SidecarName, ArchiveName, Error
		);

		return Err(Error.into());
	}
	info!(
		"      [{}/{}] Extracting core binaries and modules...",
		Task.TauriTargetTriple, Task.SidecarName
	);

	if let Err(Error) = ExtractArchive(&Task.ArchiveType, &ArchivePath, TempDirectory.path(), &Task.ExtractedFolderName)
	{
		error!(
			"      [{}/{}] Failed to extract {}: {}",
			Task.TauriTargetTriple, Task.SidecarName, ArchiveName, Error
		);

		return Err(Error.into());
	}

	let ExtractedPath = TempDirectory.path().join(&Task.ExtractedFolderName);

	if !ExtractedPath.exists() {
		let ErrorMessage = format!("      Could not find extracted folder: {:?}", ExtractedPath);

		error!("{}", ErrorMessage);

		return Err(anyhow!(ErrorMessage));
	}

	// If the destination directory already exists (from a previous version), remove
	// it.
	if Task.DestinationDirectory.exists() {
		info!("      Removing old version at: {:?}", Task.DestinationDirectory);

		fs::remove_dir_all(&Task.DestinationDirectory)?;
	}

	// Create the destination directory before moving contents into it.
	fs::create_dir_all(&Task.DestinationDirectory)?;

	info!("      Installing to: {:?}", Task.DestinationDirectory);

	// This replaces the `fs::rename` loop which fails on Windows when the temp
	// directory and destination directory are on different drives.
	let mut Options = FsExtraCopyOptions::new();
	Options.content_only = true;
	FsExtraDir::move_dir(&ExtractedPath, &Task.DestinationDirectory, &Options).with_context(|| {
		format!(
			"Failed to move contents from {:?} to {:?}",
			ExtractedPath, Task.DestinationDirectory
		)
	})?;

	// Update the cache with the new version.
	let CacheKey = format!("{}/{}/{}", Task.TauriTargetTriple, Task.SidecarName, Task.MajorVersion);

	let mut LockedCache = Cache.lock().unwrap();

	LockedCache.Entries.insert(CacheKey, Task.FullVersion.clone());

	info!(
		"    v{} ({}) for '{}' is now up to date.",
		Task.MajorVersion, Task.FullVersion, Task.TauriTargetTriple
	);

	// TempDirectory is automatically cleaned up when it goes out of scope here.
	Ok(())
}

/// Sets up the global logger for the application.
pub fn Logger() {
	let LevelText = env::var(LogEnv).unwrap_or_else(|_| "info".to_string());

	let LogLevel = LevelText.parse::<LevelFilter>().unwrap_or(LevelFilter::Info);

	env_logger::Builder::new()
		.filter_level(LogLevel)
		.format(|Buffer, Record| {
			let LevelStyle = match Record.level() {
				log::Level::Error => "ERROR".red().bold(),

				log::Level::Warn => "WARN".yellow().bold(),

				log::Level::Info => "INFO".green(),

				log::Level::Debug => "DEBUG".blue(),

				log::Level::Trace => "TRACE".magenta(),
			};

			writeln!(Buffer, "[{}] [{}]: {}", "Download".red(), LevelStyle, Record.args())
		})
		.parse_default_env()
		.init();
}

#[tokio::main]
pub async fn Fn() -> Result<()> {
	Logger();

	info!("Starting Universal Sidecar vendoring process...");

	// --- Setup ---
	let BaseSidecarDirectory = GetBaseSidecarDirectory()?;

	let CachePath = BaseSidecarDirectory.join("Cache.json");

	fs::create_dir_all(&BaseSidecarDirectory).context("Failed to create base sidecar directory.")?;

	let Cache = Arc::new(Mutex::new(DownloadCache::Load(&CachePath)));

	let HttpClient = Client::new();

	let PlatformMatrix = GetPlatformMatrix();

	let SidecarsToFetch = GetSidecarsToFetch();

	// Fetch Node versions once to be used by all tasks.
	let NodeVersions = FetchNodeVersions(&HttpClient).await?;

	let mut TasksToRun = Vec::new();

	// --- Task Generation Phase (Sequential) ---
	// First, we determine which downloads are necessary by checking the cache.
	for Platform in &PlatformMatrix {
		info!("--- Processing architecture: '{}' ---", Platform.TauriTargetTriple);

		for (SidecarName, MajorVersions) in &SidecarsToFetch {
			info!("  -> Processing sidecar: '{}'", SidecarName);

			for MajorVersion in MajorVersions {
				let DestinationDirectory = BaseSidecarDirectory
					.join(&Platform.TauriTargetTriple)
					.join(SidecarName)
					.join(MajorVersion);

				// --- Sidecar-Specific Download Logic ---
				if SidecarName == "NODE" {
					let FullVersion = match ResolveLatestPatchVersion(MajorVersion, &NodeVersions) {
						Some(Version) => Version,

						None => {
							warn!(
								"      Could not resolve a specific version for Node.js v{}. Skipping.",
								MajorVersion
							);

							continue;
						},
					};

					// Check cache to see if we need to download/update.
					let CacheKey = format!("{}/{}/{}", &Platform.TauriTargetTriple, SidecarName, MajorVersion);

					let CachedVersion = Cache.lock().unwrap().Entries.get(&CacheKey).cloned();

					if Some(FullVersion.clone()) == CachedVersion {
						info!("    v{} ({}) is already up to date, skipping.", MajorVersion, FullVersion);

						continue;
					}

					if CachedVersion.is_some() {
						info!(
							"    Found newer patch for v{}: {} -> {}. Scheduling update.",
							MajorVersion,
							CachedVersion.unwrap(),
							FullVersion
						);
					} else {
						info!("    Processing v{} (resolved to {})...", MajorVersion, FullVersion);
					}

					let ArchiveExtension = &Platform.ArchiveExtension;

					let ArchiveName =
						format!("node-{}-{}.{}", FullVersion, Platform.DownloadIdentifier, ArchiveExtension);

					let DownloadURL = format!("https://nodejs.org/dist/{}/{}", FullVersion, ArchiveName);

					let ExtractedFolderName = format!("node-{}-{}", FullVersion, Platform.DownloadIdentifier);

					let Task = DownloadTask {
						SidecarName:SidecarName.clone(),

						MajorVersion:MajorVersion.clone(),

						FullVersion,

						DownloadURL,

						DestinationDirectory,

						ArchiveType:if ArchiveExtension == "zip" { ArchiveType::Zip } else { ArchiveType::TarGz },

						ExtractedFolderName,

						TauriTargetTriple:Platform.TauriTargetTriple.clone(),
					};

					TasksToRun.push(Task);
				}
				// To add Deno, you would add an `else if SidecarName == "DENO"`
				// block here.
			}
		}
	}

	// --- Concurrent Execution Phase ---
	if TasksToRun.is_empty() {
		info!("All sidecar binaries are already up to date.");
	} else {
		info!("Found {} tasks to run. Starting concurrent downloads...", TasksToRun.len());

		// Limit to 8 concurrent jobs or num CPUs, whichever is smaller.
		let NumberOfConcurrentJobs = num_cpus::get().min(8);

		// Spawn a Tokio task for each download.
		// Run tasks concurrently.
		let Results = stream::iter(TasksToRun)
			.map(|Task| {
				let Client = HttpClient.clone();

				let Cache = Arc::clone(&Cache);

				tokio::spawn(async move { ProcessDownloadTask(Task, Client, Cache).await })
			})
			.buffer_unordered(NumberOfConcurrentJobs)
			.collect::<Vec<_>>()
			.await;

		// Check for any errors that occurred during the concurrent tasks.
		let mut ErrorsEncountered = 0;

		for Result in Results {
			// The first result is from tokio::spawn, the second from our function
			if let Err(JoinError) = Result {
				error!("A download task panicked or was cancelled: {}", JoinError);

				ErrorsEncountered += 1;
			} else if let Ok(Err(AppError)) = Result {
				// We already logged the error inside `ProcessDownloadTask`, so just count it.
				// Re-logging here to ensure it's captured at a higher level if needed.
				error!("A download task failed: {}", AppError);

				ErrorsEncountered += 1;
			}
		}

		if ErrorsEncountered > 0 {
			error!("Completed with {} errors.", ErrorsEncountered);
		}
	}

	// --- Finalization ---
	info!("Saving updated cache...");

	Cache.lock().unwrap().Save(&CachePath)?;

	info!("All sidecar binaries have been successfully processed and organized.");

	Ok(())
}

/// Main executable function.
#[allow(unused)]
fn main() {
	// We use a block here to handle the Result from Fn.
	if let Err(Error) = Fn() {
		// The logger should already be initialized by Fn, so we can use it.
		error!("The application encountered a fatal error: {}", Error);

		std::process::exit(1);
	}
}

// --- Imports ---
use std::{
	collections::HashMap,
	env,
	fs::{self, File},
	io::{self, Write},
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use colored::*;
use fs_extra::{dir as FsExtraDir, dir::CopyOptions as FsExtraCopyOptions};
use futures::stream::{self, StreamExt};
use log::{LevelFilter, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tempfile::Builder;
