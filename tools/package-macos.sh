#!/bin/bash
# Build "BW Darkroom.app" and a drag-to-Applications DMG for macOS (Apple Silicon).
#
#   tools/package-macos.sh            → dist/BW Darkroom.app, dist/BW-Darkroom-<version>-arm64.dmg
#
# Uses only tools that ship with macOS (sips, iconutil, codesign, hdiutil). The app is
# ad-hoc signed, not signed with a Developer ID or notarised: on another Mac, open it
# the first time with right-click → Open (or System Settings → Privacy & Security →
# Open Anyway). Set SIGN_IDENTITY="Developer ID Application: …" to sign properly; then
# notarise the DMG with `xcrun notarytool submit … --wait` and `xcrun stapler staple`.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$(pwd)
APP_NAME="BW Darkroom"
BIN=bw-darkroom
BUNDLE_ID="io.github.catskinroyale.bwdarkroom"
VERSION=$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version *= *"\(.*\)"/\1/p;}' Cargo.toml)
ARCH=$(uname -m)
DIST="$ROOT/dist"
APP="$DIST/$APP_NAME.app"
DMG="$DIST/BW-Darkroom-$VERSION-$ARCH.dmg"
SIGN_IDENTITY="${SIGN_IDENTITY:--}"

echo "==> Building $APP_NAME $VERSION ($ARCH)"
cargo build --profile dist -p bw-desktop

echo "==> Assembling $APP"
rm -rf "$APP" "$DMG"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp "target/dist/$BIN" "$APP/Contents/MacOS/$BIN"

# Icon: render every size from the 1024 px master.
ICONSET=$(mktemp -d)/AppIcon.iconset
mkdir -p "$ICONSET"
[ -f packaging/macos/icon_1024.png ] || python3 -I tools/make_icon.py packaging/macos/icon_1024.png
for size in 16 32 128 256 512; do
  sips -z $size $size packaging/macos/icon_1024.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) packaging/macos/icon_1024.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

sed -e "s/@VERSION@/$VERSION/g" -e "s/@BUNDLE_ID@/$BUNDLE_ID/g" -e "s/@BIN@/$BIN/g" \
  packaging/macos/Info.plist.in > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# Licence texts and project notes travel with the app.
cp README.md CHANGELOG.md LICENSE-APACHE LICENSE-MIT "$APP/Contents/Resources/"

echo "==> Signing (identity: ${SIGN_IDENTITY})"
codesign --force --options runtime --timestamp=none --sign "$SIGN_IDENTITY" "$APP"
codesign --verify --strict --verbose=1 "$APP"

echo "==> Creating $DMG"
STAGE=$(mktemp -d)
cp -R "$APP" "$STAGE/"
ln -s /Applications "$STAGE/Applications"
hdiutil create -quiet -volname "$APP_NAME $VERSION" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
rm -rf "$STAGE" "$(dirname "$ICONSET")"

echo "==> Done"
du -sh "$APP" "$DMG"
