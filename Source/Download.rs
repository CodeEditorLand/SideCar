#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	dead_code,
	unused_imports,
	unused_variables,
	unused_assignments
)]

//! ==============================================================================
//! Universal Sidecar Vendor - Rust Edition
//!
//! This program automates downloading and organizing full distributions of
//! various sidecar runtimes (like Node.js) for a Tauri application. It is a
//! Rust rewrite of the original shell script, enhanced with modern features.
//!
//! Key Features:
//!   - Asynchronous, Concurrent Downloads: Leverages Tokio to download multiple
//!     binaries in parallel, significantly speeding up the process.
//!   - Intelligent Caching: Maintains a `Cache.json` file to track downloaded
//!     versions. It automatically detects if a newer patch version is available
//!     for a requested major version and updates the binary.
//!   - Git LFS Management: Automatically creates or updates the
//!     `.gitattributes` file to ensure large binaries are tracked by Git LFS.
//!   - Extensible Design: Easily configured to support new sidecars, versions,
//!     and platforms.
//!   - Robust Error Handling: Uses `anyhow` for clear and concise error
//!     reporting.
//!   - Preserved File Structure: The final output directory structure remains
//!     identical to the original script (`Architecture/SidecarName/Version`).
//!
//! ==============================================================================

// --- Type Definitions and Structs ---

/// Represents a single platform target for which binaries will be downloaded.
/// This struct holds all the necessary identifiers for a given platform.
#[derive(Clone, Debug)]
struct PlatformTarget {
	/// The identifier used in the download URL (e.g., "win-x64",
	/// "linux-arm64").
	DownloadIdentifier:String,

	/// The file extension of the archive (e.g., "zip", "tar.gz").
	ArchiveExtension:String,

	/// The official Tauri target triple for this platform (e.g.,
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

	/// The directory where temporary folders for this task will be created.
	TempParentDirectory:PathBuf,

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
	/// The JSON is pretty-printed with tabs for indentation.
	/// Entries are sorted alphabetically by key for consistency.
	fn Save(&self, CachePath:&Path) -> Result<()> {
		// Create a BTreeMap to sort entries alphabetically by key
		let SortedEntries:BTreeMap<_, _> = self.Entries.iter().collect();

		// Create a temporary struct to hold the sorted entries for serialization
		let CacheToSerialize = serde_json::json!({

			"Entries": SortedEntries
		});

		// Create an in-memory buffer to write the serialized JSON to.
		let mut Buffer = Vec::new();

		// Create a formatter that uses a tab character for indentation.
		let Formatter = serde_json::ser::PrettyFormatter::with_indent(b"	");

		// Create a serializer with our custom formatter.
		let mut Serializer = serde_json::Serializer::with_formatter(&mut Buffer, Formatter);

		// Serialize the sorted cache data into the buffer.
		CacheToSerialize.serialize(&mut Serializer)?;

		// Write the buffer's contents to the actual file on disk.
		fs::write(CachePath, &Buffer)
			.with_context(|| format!("Failed to write tab-formatted cache to {:?}", CachePath))?;

		Ok(())
	}
}

// --- Configuration ---

/// Returns the root directory where all sidecars will be stored.
/// This is determined dynamically by navigating up from the executable's
/// location and detecting the SideCar project root. It handles both:
/// - Standalone builds: `.../SideCar/Target/release/`
/// - Workspace builds: `.../workspace/Target/release/SideCar` (where the
///   workspace root contains multiple crates including Element/SideCar)
fn GetBaseSidecarDirectory() -> Result<PathBuf> {
	// Get the full path to the currently running executable.
	let CurrentExePath = env::current_exe().context("Failed to get the path of the current executable.")?;

	// Start from the directory containing the executable and walk up the tree.
	let mut CurrentDir = CurrentExePath
		.parent()
		.context("Executable must be in a directory (not the root).")?;

	loop {
		// Check A: Does Source/Library.rs exist in current directory? → return current
		// directory
		let LibraryRsPath = CurrentDir.join("Source").join("Library.rs");

		if LibraryRsPath.exists() {
			return Ok(CurrentDir.to_path_buf());
		}

		// Check B: Does a Cargo.toml exist in current directory with package.name =
		// "SideCar"? → return current directory
		let CargoTomlPath = CurrentDir.join("Cargo.toml");

		if CargoTomlPath.exists() {
			if let Ok(CargoContents) = fs::read_to_string(&CargoTomlPath) {
				if let Ok(Toml) = toml::from_str::<toml::Value>(&CargoContents) {
					if let Some(Package) = Toml.get("package") {
						if let Some(PackageName) = Package.get("name").and_then(|v| v.as_str()) {
							if PackageName == "SideCar" {
								// Verify that Source subdirectory exists as additional confirmation.
								let SourceDir = CurrentDir.join("Source");

								if SourceDir.exists() {
									return Ok(CurrentDir.to_path_buf());
								}
							}
						}
					}
				}
			}
		}

		// Check C: Does Element/SideCar/Cargo.toml exist relative to current directory
		// AND does it have package.name = "SideCar"? → return Element/SideCar
		// subdirectory path
		let SubdirCargoTomlPath = CurrentDir.join("Element").join("SideCar").join("Cargo.toml");

		if SubdirCargoTomlPath.exists() {
			if let Ok(CargoContents) = fs::read_to_string(&SubdirCargoTomlPath) {
				if let Ok(Toml) = toml::from_str::<toml::Value>(&CargoContents) {
					if let Some(Package) = Toml.get("package") {
						if let Some(PackageName) = Package.get("name").and_then(|v| v.as_str()) {
							if PackageName == "SideCar" {
								// Verify that the Element/SideCar/Source subdirectory exists.
								let SourceDir = CurrentDir.join("Element").join("SideCar").join("Source");

								if SourceDir.exists() {
									// Return the full path to the Element/SideCar subdirectory.
									return Ok(CurrentDir.join("Element").join("SideCar"));
								}
							}
						}
					}
				}
			}
		}

		// Move up one level.
		let NextDir = match CurrentDir.parent() {
			Some(Parent) => Parent,

			None => break, // Reached filesystem root without finding the project
		};

		CurrentDir = NextDir;
	}

	Err(anyhow!(
		"Could not determine the SideCar base directory. The executable should be built from within the SideCar crate \
		 or from the workspace containing Element/SideCar. Searched up from: {}",
		CurrentExePath.display()
	))
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

/// Environment variable that controls the log level filter.
///
/// Reads the `RUST_LOG` environment variable. Defaults to `"info"` if unset.
/// Supported levels: `error`, `warn`, `info`, `debug`, `trace`.
pub const LogEnv:&str = "RUST_LOG";

/// Manages the `.gitattributes` file to ensure binaries are tracked by Git LFS.
/// If the file does not exist, it is created. If it exists, missing rules are
/// appended.
fn UpdateGitattributes(BaseDirectory:&Path) -> Result<()> {
	const GITATTRIBUTES_HEADER:&str = r#"################################################################################
# Git LFS configuration for vendored Tauri Sidecars
#
# This file tells Git to use LFS (Large File Storage) for the heavy binary
# files and modules downloaded by the sidecar vendoring script. This keeps the
# main repository history small and fast.
#
# The `-text` attribute is used to prevent Git from normalizing line endings,

# which is critical for binary files and scripts.
#
# This file is automatically managed by the sidecar vendor script.
################################################################################

# --- Rule Definitions ---"#;

	const GITATTRIBUTES_RULES:&[&str] = &[
		"**/NODE/**/bin/node filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/node.exe filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/bin/npm filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/bin/npx filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/bin/corepack filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/npm filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/npm.cmd filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/npx filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/npx.cmd filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/corepack filter=lfs diff=lfs merge=lfs -text",
		"**/NODE/**/corepack.cmd filter=lfs diff=lfs merge=lfs -text",
		"",
		"# --- Rules for the SideCar build artifacts ---",
		"",
		"Target/debug/*.exe filter=lfs diff=lfs merge=lfs -text",
		"Target/release/*.exe filter=lfs diff=lfs merge=lfs -text",
		"",
		"Target/debug/SideCar filter=lfs diff=lfs merge=lfs -text",
		"Target/release/SideCar filter=lfs diff=lfs merge=lfs -text",
		"",
		"Target/debug/Download filter=lfs diff=lfs merge=lfs -text",
		"Target/release/Download filter=lfs diff=lfs merge=lfs -text",
	];

	let GitattributesPath = BaseDirectory.join(".gitattributes");

	if !GitattributesPath.exists() {
		info!("Creating .gitattributes file to track binaries with Git LFS.");

		let mut File = File::create(&GitattributesPath)
			.with_context(|| format!("Failed to create .gitattributes file at {:?}", GitattributesPath))?;

		writeln!(File, "{}", GITATTRIBUTES_HEADER)?;

		for Rule in GITATTRIBUTES_RULES {
			// This will write a blank line for any empty strings in the array
			writeln!(File, "{}", Rule)?;
		}
	} else {
		info!(".gitattributes file found. Verifying LFS rules...");

		let Content = fs::read_to_string(&GitattributesPath)?;

		let MissingRules:Vec<_> = GITATTRIBUTES_RULES
			.iter()

			// Filter out blank lines and comments from the check
			.filter(|rule| !rule.is_empty() && !rule.starts_with('#'))
			.filter(|rule| !Content.contains(*rule))
			.collect();

		if !MissingRules.is_empty() {
			info!("Adding {} missing LFS rules to .gitattributes.", MissingRules.len());

			let mut File = fs::OpenOptions::new()
				.append(true)
				.open(&GitattributesPath)
				.with_context(|| format!("Failed to open .gitattributes for appending at {:?}", GitattributesPath))?;

			writeln!(File, "\n\n# --- Rules Automatically Added by Vendor Script ---")?;

			for Rule in MissingRules {
				writeln!(File, "{}", Rule)?;
			}
		} else {
			info!(".gitattributes is already up to date.");
		}
	}

	Ok(())
}

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
/// This function now performs a full extraction to ensure a complete
/// distribution.
fn ExtractArchive(ArchiveType:&ArchiveType, ArchivePath:&Path, ExtractionDirectory:&Path) -> Result<()> {
	info!("Performing a full extraction of the archive...");

	match ArchiveType {
		ArchiveType::Zip => {
			let File = File::open(ArchivePath)?;

			let mut Archive = zip::ZipArchive::new(File)?;

			Archive.extract(ExtractionDirectory)?;
		},

		ArchiveType::TarGz => {
			let File = File::open(ArchivePath)?;

			let Decompressor = flate2::read::GzDecoder::new(File);

			let mut Archive = tar::Archive::new(Decompressor);

			Archive.unpack(ExtractionDirectory)?;
		},
	}

	Ok(())
}

/// The main asynchronous function for processing a single download task.
/// This function is designed to be run concurrently for multiple tasks.
async fn ProcessDownloadTask(Task:DownloadTask, Client:Client, Cache:Arc<Mutex<DownloadCache>>) -> Result<()> {
	// Create the temporary directory inside the designated "Temporary" subfolder.
	let TempDirectory = Builder::new()
		.prefix("SideCar-Download-")
		.tempdir_in(&Task.TempParentDirectory)
		.context("Failed to create temporary directory.")?;

	let ArchiveName = Task.DownloadURL.split('/').last().unwrap_or("Download.tmp");

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

	info!("      [{}/{}] Extracting archive...", Task.TauriTargetTriple, Task.SidecarName);

	if let Err(Error) = ExtractArchive(&Task.ArchiveType, &ArchivePath, TempDirectory.path()) {
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

	// If the destination directory already exists, remove it.
	if Task.DestinationDirectory.exists() {
		info!("      Removing old version at: {:?}", Task.DestinationDirectory);

		fs::remove_dir_all(&Task.DestinationDirectory)?;
	}

	// Ensure the parent of the final destination exists.
	if let Some(Parent) = Task.DestinationDirectory.parent() {
		fs::create_dir_all(Parent)?;
	}

	info!("      Installing to: {:?}", Task.DestinationDirectory);

	fs::rename(&ExtractedPath, &Task.DestinationDirectory).with_context(|| {
		format!(
			"Failed to rename/move extracted directory from {:?} to {:?}",
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

	Ok(())
}

/// Initialises the coloured terminal logger for the application.
///
/// Reads the log level from the [`LogEnv`] environment variable (defaults to
/// `"info"`), and configures [`env_logger`] with a custom format that prefixes
/// each line with a coloured `[Download]` tag and the log level.
///
/// Must be called once at process start before any logging macros.
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

/// Main entry point for the sidecar vendoring pipeline.
///
/// Initialises telemetry via [`CommonLibrary::Telemetry`], sets up the logger,
/// then orchestrates the full download workflow:
///
/// 1. Resolves the base sidecar directory from configuration.
/// 2. Updates `.gitattributes` for Git LFS tracking.
/// 3. Creates a temporary downloads directory.
/// 4. Loads or initialises the download cache (`Cache.json`).
/// 5. Fetches the platform matrix and sidecar definitions.
/// 6. Downloads all required binaries concurrently.
/// 7. Extracts, verifies, and places each binary in the correct output
///    directory, cleaning up temporary files on completion.
///
/// # Errors
///
/// Returns an [`anyhow::Error`] if any step in the pipeline fails - network
/// errors, checksum mismatches, filesystem issues, etc.
///
/// # Panics
///
/// Panics if the Tokio runtime fails to initialise (the `#[tokio::main]`
/// attribute handles runtime creation).
#[tokio::main]
pub async fn Fn() -> Result<()> {
	// [Boot] [Telemetry] Bring up shared dual-pipe (PostHog + OTLP).
	// No-op in release builds and when `Capture=false`.
	CommonLibrary::Telemetry::Initialize::Fn(CommonLibrary::Telemetry::Tier::Tier::SideCar).await;

	Logger();

	info!("Starting Universal Sidecar vendoring process...");

	// --- Setup ---
	let BaseSidecarDirectory = GetBaseSidecarDirectory()?;

	// Manage the .gitattributes file for Git LFS.
	UpdateGitattributes(&BaseSidecarDirectory)?;

	// Define and create the dedicated directory for temporary downloads.
	let TempDownloadsDirectory = BaseSidecarDirectory.join("Temporary");

	fs::create_dir_all(&TempDownloadsDirectory)
		.with_context(|| format!("Failed to create temporary directory at {:?}", TempDownloadsDirectory))?;

	let CachePath = BaseSidecarDirectory.join("Cache.json");

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

						TempParentDirectory:TempDownloadsDirectory.clone(),

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
	collections::{BTreeMap, HashMap},
	env,
	fs::{self, File},
	io::Write,
	path::{Path, PathBuf},
	sync::{Arc, Mutex},
};

use anyhow::{Context, Result, anyhow};
use colored::*;
use futures::stream::{self, StreamExt};
use log::{LevelFilter, error, info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tempfile::Builder;
use toml;
