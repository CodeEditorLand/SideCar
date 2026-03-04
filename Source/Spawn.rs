// DEPENDENCY: This module file was created to resolve a missing module error.
// The original Spawn.rs was located at Source/Source/SideCar/Spawn.rs in an unusual directory structure.
// Consider refactoring the directory structure to avoid duplicate/confusing module locations.

#[allow(unused_imports)]
use tauri::{AppHandle, Manager};
use std::fs;
// Note: Mist crate has lib name "mist", so we use lowercase for imports
use mist::dns_port;
// DEPENDENCY: Add tauri-plugin-shell to Cargo.toml dependencies for Tauri 2.x shell support
use tauri_plugin_shell::ShellExt;

const DNS_OVERRIDE: &str = include_str!("../Resource/dns-override.js");

/// Spawns a Node.js sidecar with DNS override configured to use the local Hickory DNS server.
///
/// This function:
/// 1. Creates the app data directory if it doesn't exist
/// 2. Writes the DNS override JavaScript file to the app data directory
/// 3. Configures the sidecar process with NODE_OPTIONS to require the DNS override script
/// 4. Sets the LAND_DNS_SERVER environment variable with the local DNS server address
/// 5. Spawns the sidecar process
///
/// # Parameters
///
/// * `app` - The Tauri app handle, used to access the app data directory and shell
/// * `sidecar_name` - The name of the sidecar executable to spawn
///
/// # Returns
///
/// Returns `Ok(())` if the sidecar was spawned successfully, or an error if:
/// - The app data directory couldn't be created or accessed
/// - The DNS override file couldn't be written
/// - The sidecar couldn't be found or spawned
///
/// # Example
///
/// ```rust,no_run
/// use tauri::Manager;
/// use SideCar::Spawn::spawn_node_sidecar;
///
/// #[tauri::command]
/// fn launch_sidecar(app: tauri::AppHandle) -> Result<(), String> {
///     spawn_node_sidecar(&app, "my-sidecar")
///         .map_err(|e| e.to_string())?;
///     Ok(())
/// }
/// ```
#[allow(dead_code)]
pub fn spawn_node_sidecar(
    app: &AppHandle,
    sidecar_name: &str,
) -> anyhow::Result<()> {
    // Ensure app data directory exists
    let data_dir = app.path().app_data_dir()?;
    fs::create_dir_all(&data_dir)?;

    // Write DNS override script to app data directory
    let override_path = data_dir.join("dns-override.js");
    fs::write(&override_path, DNS_OVERRIDE)?;

    // Get the DNS server port from Mist module
    let port = dns_port();
    let node_opts = format!("--require {}", override_path.display());

    // Spawn the sidecar with DNS configuration
    app.shell()
        .sidecar(sidecar_name)?
        .env("NODE_OPTIONS",    &node_opts)
        .env("LAND_DNS_SERVER", format!("127.0.0.1:{port}"))
        .spawn()?;

    Ok(())
}
