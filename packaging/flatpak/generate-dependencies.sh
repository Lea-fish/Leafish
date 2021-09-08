#!/usr/bin/env bash

# Bash script to generate a manifest containing dependencies for Rust.
# If it works, it outputs exactly in $LEAFISH_REPO/packaging/flatpak/cargo-sources.json .

git submodule update --init

# Check if user is in root of repo directory
if [[ -d packaging ]]; then
	FLATPAK_DIR="packaging/flatpak"
	CARGO_LOCK_DIR="."
# Check if user is in 'packaging' directory
elif [[ -d flatpak ]]; then
	FLATPAK_DIR="flatpak"
	CARGO_LOCK_DIR=".."
# Check if user is in 'packaging/flatpak' directory
elif [[ -d flatpak-builder-tools ]]; then
	FLATPAK_DIR="."
	CARGO_LOCK_DIR="../.."
fi

python "$FLATPAK_DIR/flatpak-builder-tools/cargo/flatpak-cargo-generator.py" "$CARGO_LOCK_DIR/Cargo.lock" -o "$FLATPAK_DIR/cargo-sources.json"
