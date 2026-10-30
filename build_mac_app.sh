#!/bin/bash
set -euo pipefail

APP_NAME="Bird Player"
RELEASE_DIR="target/${CARGO_BUILD_TARGET:+$CARGO_BUILD_TARGET/}release"
APP_DIR="$RELEASE_DIR/$APP_NAME.app"
CONTENTS_DIR="$APP_DIR/Contents"
MACOS_DIR="$CONTENTS_DIR/MacOS"
RESOURCES_DIR="$CONTENTS_DIR/Resources"

echo "Building release binary..."
cargo build --locked --release --features production-data
APP_VERSION=$(cargo metadata --locked --no-deps --format-version 1 | python3 -c '
import json, sys
metadata = json.load(sys.stdin)
package = next(p for p in metadata["packages"] if p["name"] == "bird-player")
print(package["version"].split("-")[0].split("+")[0])
')

echo "Creating app bundle structure..."
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

echo "Copying binary..."
cp "$RELEASE_DIR/bird-player" "$MACOS_DIR/"

echo "Copying icon..."
if [ -f "assets/icons/icon.icns" ]; then
    cp assets/icons/icon.icns "$RESOURCES_DIR/"
fi

echo "Creating Info.plist..."
cat << PLIST > "$CONTENTS_DIR/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>English</string>
    <key>CFBundleExecutable</key>
    <string>bird-player</string>
    <key>CFBundleIconFile</key>
    <string>icon.icns</string>
    <key>CFBundleIdentifier</key>
    <string>com.retric.birdplayer</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>Bird Player</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>$APP_VERSION</string>
    <key>CFBundleVersion</key>
    <string>$APP_VERSION</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
PLIST

echo "App bundle created at $APP_DIR"
