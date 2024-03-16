#!/bin/bash

cargo build --release --target x86_64-pc-windows-gnu || { echo "Error building Windows client"; exit 1; }
# cargo build --release --target x86_64-unknown-linux-gnu || { echo "Error building Linux client"; exit 1; }
cargo build --release || { echo "Error building Linux client"; exit 1; }

cp ./target/x86_64-pc-windows-gnu/release/leafish.exe ./target/leafish_x86_64_windows.exe || { echo "Error copying Windows client"; exit 1; }
cp ./target/release/leafish ./target/leafish_x86_64_linux || { echo "Error copying Linux client"; exit 1; }

echo "Successfully built clients"