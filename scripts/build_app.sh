#!/usr/bin/env bash
# Builds a universal (Apple Silicon + Intel) MacPulse.app. Run on a Mac.
set -euo pipefail
cd "$(dirname "$0")/.."

rustup target add aarch64-apple-darwin x86_64-apple-darwin
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin

APP=dist/MacPulse.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
lipo -create -output "$APP/Contents/MacOS/macpulse" \
  target/aarch64-apple-darwin/release/macpulse \
  target/x86_64-apple-darwin/release/macpulse
cp resources/Info.plist "$APP/Contents/Info.plist"
codesign --force --deep --sign - "$APP"   # ad-hoc signature, fine for running on your own Mac
echo "Built $APP"
echo "Install:  cp -R $APP /Applications/"
