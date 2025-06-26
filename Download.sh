#!/usr/bin/env bash

# ==============================================================================
# Universal Sidecar Vendor Script
#
# This script automates downloading and organizing full distributions of various
# sidecar runtimes (like Node.js, Deno, etc.) for a Tauri application.
# It is designed to be extensible for new sidecars and versions.
#
# Key Features:
#   - Downloads full distributions, including package managers like npm.
#   - Organizes binaries in a structured `Architecture/SidecarName/Version` layout.
#   - Skips downloads if the target directory already exists.
#
# Prerequisites:
#   - curl: For downloading files.
#   - jq: For parsing JSON from release indexes.
#   - tar, unzip: For extracting archives.
#
# ==============================================================================

# --- Configuration ---

# Exit immediately if a command exits with a non-zero status.
set -e
# Exit immediately if a pipeline command fails.
set -o pipefail

# The root directory where all sidecars will be stored.
BASE_SIDECAR_DIR="/d/Developer/Application/CodeEditorLand/Land/Element/SideCar"

# Platform matrix:
# Format: "DOWNLOAD_IDENTIFIER:ARCHIVE_EXTENSION:TAURI_TARGET_TRIPLE"

PLATFORM_MATRIX=(
	"win-x64:zip:x86_64-pc-windows-msvc"

	"linux-x64:tar.gz:x86_64-unknown-linux-gnu"

	"linux-arm64:tar.gz:aarch64-unknown-linux-gnu"

	"darwin-x64:tar.gz:x86_64-apple-darwin"

	"darwin-arm64:tar.gz:aarch64-apple-darwin"

)

# --- Sidecar Definitions ---
# Use an associative array to define which sidecars and versions to fetch.
# This makes it easy to add more sidecars like Deno in the future.
declare -A SIDECARS
SIDECARS["NODE"]="24 23 22 21 20 19 18 17 16"

# To add Deno later, you would just add a line like:
# SIDECARS["Deno"]="1.40.0 1.39.0"

# --- Helper Functions ---

# ANSI color codes for logging
C_BLUE="\033[34m"

C_GREEN="\033[32m"

C_YELLOW="\033[33m"

C_RED="\033[31m"

C_RESET="\033[0m"

log_info() {
	echo -e "[${C_BLUE}INFO${C_RESET}]: $1"

}

log_success() {
	echo -e "[${C_GREEN}SUCCESS${C_RESET}]: $1"

}

log_warn() {
	echo -e "[${C_YELLOW}WARN${C_RESET}]: $1"

}

log_error() {
	echo -e "[${C_RED}ERROR${C_RESET}]: $1" >&2
}

# --- Main Logic ---

log_info "Starting Universal Sidecar vendoring process..."

# Fetch the official Node.js versions index once
log_info "Fetching Node.js version index for resolving versions..."

NODE_VERSIONS_JSON=$(curl -sS https://nodejs.org/dist/index.json)

if [ -z "$NODE_VERSIONS_JSON" ]; then
	log_error "Failed to fetch Node.js version index. Check network connection."

	exit 1
fi

# Process each platform architecture first
for PLATFORM_INFO in "${PLATFORM_MATRIX[@]}"; do
	# Parse platform info string
	IFS=':' read -r DOWNLOAD_ARCH FILE_EXT TAURI_TRIPLE <<<"$PLATFORM_INFO"

	log_info "--- Processing architecture: '$TAURI_TRIPLE' ---"

	# Loop through each defined sidecar (e.g., "NODE", "Deno")

	for SIDECAR_NAME in "${!SIDECARS[@]}"; do
		log_info "  -> Processing sidecar: '$SIDECAR_NAME'"

		# Get the list of versions for the current sidecar
		VERSIONS_TO_FETCH="${SIDECARS[$SIDECAR_NAME]}"

		for MAJOR_VERSION in $VERSIONS_TO_FETCH; do
			# The final destination directory for the full distribution
			DEST_DIR="$BASE_SIDECAR_DIR/$TAURI_TRIPLE/$SIDECAR_NAME/$MAJOR_VERSION"

			# Skip if the final directory already exists, assuming it's complete.
			if [ -d "$DEST_DIR" ]; then
				log_success "    v$MAJOR_VERSION already exists, skipping: $DEST_DIR"

				continue
			fi

			log_info "    Processing v$MAJOR_VERSION..."

			# --- Sidecar-Specific Download Logic ---
			DOWNLOAD_URL=""

			EXTRACTED_FOLDER_NAME=""

			if [ "$SIDECAR_NAME" == "NODE" ]; then
				VERSION_PREFIX="v$MAJOR_VERSION."

				FULL_VERSION=$(echo "$NODE_VERSIONS_JSON" | jq -r --arg ver "$VERSION_PREFIX" '.[] | select(.version | startswith($ver)) | .version' | head -n 1)

				if [ -z "$FULL_VERSION" ]; then
					log_warn "      Could not resolve a specific version for Node.js v$MAJOR_VERSION. Skipping."

					continue
				fi

				log_info "      Resolved to latest patch: $FULL_VERSION"

				ARCHIVE_NAME="node-${FULL_VERSION}-${DOWNLOAD_ARCH}.${FILE_EXT}"

				DOWNLOAD_URL="https://nodejs.org/dist/${FULL_VERSION}/${ARCHIVE_NAME}"

				EXTRACTED_FOLDER_NAME="node-${FULL_VERSION}-${DOWNLOAD_ARCH}"

				# Example of how you would add Deno
				# elif [ "$SIDECAR_NAME" == "Deno" ]; then
				#   # Deno has a different URL structure and archive naming, handle it here
				#   log_error "Deno logic not yet implemented."

				#   continue
			fi

			if [ -z "$DOWNLOAD_URL" ]; then
				log_error "      No download logic defined for sidecar '$SIDECAR_NAME'. Skipping."

				continue
			fi

			# Create a temporary directory for the download and extraction
			TMP_DIR=$(mktemp -d)

			# Ensure the temporary directory is cleaned up on script exit
			trap 'rm -rf "$TMP_DIR"' EXIT

			log_info "      Downloading from: $DOWNLOAD_URL"

			curl -L -o "$TMP_DIR/$ARCHIVE_NAME" "$DOWNLOAD_URL" --progress-bar

			# Selectively extract only the core binaries and modules, not the entire archive.
			log_info "      Extracting core binaries and modules..."

			if [[ "$FILE_EXT" == "zip" ]]; then
				# For Windows, binaries are in the root, modules in node_modules.
				# We also grab the .cmd scripts for npm/npx/corepack to work in Windows terminals.
				unzip -q "$TMP_DIR/$ARCHIVE_NAME" \
					"$EXTRACTED_FOLDER_NAME/node.exe" \
					"$EXTRACTED_FOLDER_NAME/npm" "$EXTRACTED_FOLDER_NAME/npm.cmd" \
					"$EXTRACTED_FOLDER_NAME/npx" "$EXTRACTED_FOLDER_NAME/npx.cmd" \
					"$EXTRACTED_FOLDER_NAME/corepack" "$EXTRACTED_FOLDER_NAME/corepack.cmd" \
					"$EXTRACTED_FOLDER_NAME/node_modules/npm/*" \
					"$EXTRACTED_FOLDER_NAME/node_modules/corepack/*" \
					-d "$TMP_DIR"
			else
				# For Linux/macOS, binaries are in bin/, and modules are in lib/node_modules.
				# We extract the entire directories for npm and corepack as they contain many required files.
				tar -xzf "$TMP_DIR/$ARCHIVE_NAME" -C "$TMP_DIR" \
					"$EXTRACTED_FOLDER_NAME/bin/node" \
					"$EXTRACTED_FOLDER_NAME/bin/npm" \
					"$EXTRACTED_FOLDER_NAME/bin/npx" \
					"$EXTRACTED_FOLDER_NAME/bin/corepack" \
					"$EXTRACTED_FOLDER_NAME/lib/node_modules/npm" \
					"$EXTRACTED_FOLDER_NAME/lib/node_modules/corepack"
			fi

			# Check if the expected folder was extracted
			if [ ! -d "$TMP_DIR/$EXTRACTED_FOLDER_NAME" ]; then
				log_error "      Could not find extracted folder: $TMP_DIR/$EXTRACTED_FOLDER_NAME"

				rm -rf "$TMP_DIR"

				trap - EXIT
				continue
			fi

			# Create the final destination directory and move the *contents* of the extracted folder
			mkdir -p "$DEST_DIR"

			log_info "      Installing to: $DEST_DIR"

			mv "$TMP_DIR/$EXTRACTED_FOLDER_NAME"/* "$DEST_DIR/"

			# Clean up the temporary directory for this download
			rm -rf "$TMP_DIR"

			trap - EXIT # Clear the trap for the next iteration
		done
	done
done

log_success "All sidecar binaries have been successfully downloaded and organized."
