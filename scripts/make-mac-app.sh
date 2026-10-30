#!/usr/bin/env bash
# Puts Noob into a Mac app, "Noob.app", so it has its own icon in the Dock.
# An app is really a folder that macOS shows as one thing:
#   Noob.app/Contents/Info.plist            what the app is called, its icon, its version
#   Noob.app/Contents/MacOS/noob-app        the window
#   Noob.app/Contents/MacOS/noob            the terminal Noob (for installers)
#   Noob.app/Contents/Resources/Noob.icns   the icon
#
# Usage (after `cargo build --release --features app --target <target>`):
#   scripts/make-mac-app.sh aarch64-apple-darwin 0.4.0
set -euo pipefail

target="${1:?Usage: scripts/make-mac-app.sh <target> <version>}"
version="${2:?Usage: scripts/make-mac-app.sh <target> <version>}"
[[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "Not a version: $version" >&2; exit 1; }
[[ "$target" =~ ^[a-z0-9_]+-apple-darwin$ ]] || { echo "Not a Mac target: $target" >&2; exit 1; }
cd "$(dirname "$0")/.."

release="target/$target/release"
app="$release/Noob.app"
rm -rf "$app" # an older build of the app, made by this script
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
cp "$release/noob-app" "$release/noob" "$app/Contents/MacOS/"
cp assets/Noob.icns "$app/Contents/Resources/"

cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>Noob</string>
  <key>CFBundleDisplayName</key><string>Noob</string>
  <key>CFBundleIdentifier</key><string>io.github.anand-shekhawat.noob</string>
  <key>CFBundleExecutable</key><string>noob-app</string>
  <key>CFBundleIconFile</key><string>Noob</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>$version</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# A free "ad-hoc" signature. It isn't checked by Apple (that costs money), but Macs
# with Apple chips won't start a program that has no signature at all.
codesign --force --deep --sign - "$app"
echo "$app"
