#!/bin/bash

echo -n "Please enter the path to the installer java project: "
read path

cd bootstrap
cargo build --release --target x86_64-pc-windows-gnu || { echo "Error building Windows bootstrap"; exit 1; }
cargo build --release --target x86_64-unknown-linux-gnu || { echo "Error building Linux bootstrap"; exit 1; }

cp ./target/x86_64-pc-windows-gnu/release/bootstrap.exe ../install/resources/bootstrap_x86_64_windows.exe || { echo "Error copying Windows bootstrap"; exit 1; }
cp ./target/x86_64-unknown-linux-gnu/release/bootstrap ../install/resources/bootstrap_x86_64_linux || { echo "Error copying Linux bootstrap"; exit 1; }

cd ..
cd install

cargo build --release --target x86_64-pc-windows-gnu || { echo "Error building Windows installer"; exit 1; }
cargo build --release --target x86_64-unknown-linux-gnu || { echo "Error building Linux installer"; exit 1; }

resources_sub_path="/resources"
resources_path="$path$resources_sub_path"

if [ ! -d "$resources_path" ]; then
    echo "The provided jar path doesn't have a resources folder"
    exit 1
fi

windows_sub_path="/install_x86_64_windows.exe"
linux_sub_path="/install_x86_64_linux"

cp ./target/x86_64-pc-windows-gnu/release/install.exe "$resources_path$windows_sub_path" || { echo "Error copying Windows binary to resources"; exit 1; }
cp ./target/x86_64-unknown-linux-gnu/release/install "$resources_path$linux_sub_path" || { echo "Error copying Linux binary to resources"; exit 1; }

echo "Successfully updated installer binary resources"