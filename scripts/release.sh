#!/bin/bash
set -e

echo "Starting builds for all platforms..."

rm -rf dist .tmp_build
mkdir -p dist
mkdir -p .tmp_build

echo "Building macOS Universal Binary..."
cargo build --target aarch64-apple-darwin --release
cargo build --target x86_64-apple-darwin --release
lipo -create \
  target/aarch64-apple-darwin/release/moviebox-tui \
  target/x86_64-apple-darwin/release/moviebox-tui \
  -output .tmp_build/moviebox
tar -czf dist/MovieBox_macOS_Universal.tar.gz -C .tmp_build moviebox
rm .tmp_build/moviebox
echo "macOS Universal build complete!"

echo "Building Windows Executable..."
cargo build --target x86_64-pc-windows-gnu --release
cp target/x86_64-pc-windows-gnu/release/moviebox-tui.exe .tmp_build/MovieBox.exe
cd .tmp_build && zip -q ../dist/MovieBox_Windows_x64.zip MovieBox.exe && cd ..
rm .tmp_build/MovieBox.exe
echo "Windows build complete!"

echo "Building Linux Binary (via Docker)..."
if ! docker info > /dev/null 2>&1; then
  echo "Warning: Docker is not running! Please open the Docker app on your Mac and run this script again to build the Linux version."
else
  CROSS_CONTAINER_OPTS="--platform linux/amd64" cross build --target x86_64-unknown-linux-musl --release
  cp target/x86_64-unknown-linux-musl/release/moviebox-tui .tmp_build/moviebox
  tar -czf dist/MovieBox_Linux_x64.tar.gz -C .tmp_build moviebox
  echo "Linux build complete!"
fi

rm -rf .tmp_build

echo ""
echo "All done! Your release files are packed and ready to upload to GitHub:"
ls -lh dist
