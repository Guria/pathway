#!/usr/bin/env bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This script must be run on macOS." >&2
  exit 1
fi

require_cmd() {
  if ! command -v "$1" >/dev/null 2>&1; then
    echo "Required command '$1' not found in PATH" >&2
    exit 1
  fi
}

require_cmd rustup
require_cmd cargo
require_cmd xcrun
require_cmd swiftc
require_cmd lipo
require_cmd codesign
require_cmd ditto
require_cmd plutil
require_cmd python3
require_cmd sips
require_cmd iconutil

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
CORE_DIR="$ROOT_DIR/core"
PKG_DIR="$ROOT_DIR/packaging/macos"
BUILD_DIR="$PKG_DIR/build"
DIST_DIR="$PKG_DIR/dist"
APP_NAME="Pathway"
APP_BUNDLE="$BUILD_DIR/${APP_NAME}.app"
INFO_TEMPLATE="$PKG_DIR/Info.plist"
ENTITLEMENTS_FILE="$PKG_DIR/PathwayShim.entitlements"
SWIFT_SRC="$PKG_DIR/PathwayShim.swift"
ICON_SRC="$ROOT_DIR/assets/pathway-logo.png"
ICONSET_DIR="$BUILD_DIR/icon.iconset"
SDK_PATH="$(xcrun --sdk macosx --show-sdk-path)"

mkdir -p "$BUILD_DIR" "$DIST_DIR"
rm -rf "$APP_BUNDLE"
mkdir -p "$APP_BUNDLE/Contents/MacOS" "$APP_BUNDLE/Contents/Resources"

VERSION=$(
  cd "$CORE_DIR" && python3 - <<'PY'
import json
import subprocess

metadata = subprocess.run(
    ["cargo", "metadata", "--no-deps", "--format-version=1"],
    check=True,
    capture_output=True,
    text=True,
).stdout
data = json.loads(metadata)
packages = data.get("packages", [])
print(packages[0]["version"] if packages else "0.0.0")
PY
)

rustup target add aarch64-apple-darwin x86_64-apple-darwin

pushd "$CORE_DIR" >/dev/null
cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-apple-darwin
popd >/dev/null

TMP_UNIV="$BUILD_DIR/pathway-universal"
lipo -create \
  "$CORE_DIR/target/aarch64-apple-darwin/release/pathway" \
  "$CORE_DIR/target/x86_64-apple-darwin/release/pathway" \
  -output "$TMP_UNIV"
install -m 755 "$TMP_UNIV" "$APP_BUNDLE/Contents/Resources/pathway"
rm -f "$TMP_UNIV"

SWIFT_ARM="$BUILD_DIR/PathwayShim-arm64"
SWIFT_X86="$BUILD_DIR/PathwayShim-x86_64"
cleanup() { rm -f "$SWIFT_ARM" "$SWIFT_X86"; rm -rf "$ICONSET_DIR"; }
trap cleanup EXIT

swiftc -parse-as-library -O -sdk "$SDK_PATH" -target arm64-apple-macos11 "$SWIFT_SRC" -o "$SWIFT_ARM"
swiftc -parse-as-library -O -sdk "$SDK_PATH" -target x86_64-apple-macos10.15 "$SWIFT_SRC" -o "$SWIFT_X86"

lipo -create "$SWIFT_ARM" "$SWIFT_X86" -output "$APP_BUNDLE/Contents/MacOS/PathwayShim"
chmod +x "$APP_BUNDLE/Contents/MacOS/PathwayShim"
rm -f "$SWIFT_ARM" "$SWIFT_X86"

/usr/bin/sed "s/@VERSION@/$VERSION/g" "$INFO_TEMPLATE" > "$APP_BUNDLE/Contents/Info.plist"
plutil -lint "$APP_BUNDLE/Contents/Info.plist" >/dev/null

if [[ -f "$ICON_SRC" ]]; then
  rm -rf "$ICONSET_DIR"
  mkdir -p "$ICONSET_DIR"
  declare -a BASE_SIZES=(16 32 128 256 512)
  for SIZE in "${BASE_SIZES[@]}"; do
    sips -z "$SIZE" "$SIZE" "$ICON_SRC" --out "$ICONSET_DIR/icon_${SIZE}x${SIZE}.png" >/dev/null
    sips -z "$((SIZE * 2))" "$((SIZE * 2))" "$ICON_SRC" --out "$ICONSET_DIR/icon_${SIZE}x${SIZE}@2x.png" >/dev/null
  done
  iconutil -c icns "$ICONSET_DIR" -o "$APP_BUNDLE/Contents/Resources/icon.icns"
  rm -rf "$ICONSET_DIR"
fi

IDENTITY="${CODESIGN_IDENTITY:--}"
if [[ -f "$ENTITLEMENTS_FILE" ]]; then
  codesign --force --sign "$IDENTITY" --entitlements "$ENTITLEMENTS_FILE" "$APP_BUNDLE/Contents/Resources/pathway"
  codesign --force --sign "$IDENTITY" --entitlements "$ENTITLEMENTS_FILE" "$APP_BUNDLE/Contents/MacOS/PathwayShim"
  codesign --force --sign "$IDENTITY" --entitlements "$ENTITLEMENTS_FILE" "$APP_BUNDLE"
else
  codesign --force --sign "$IDENTITY" "$APP_BUNDLE/Contents/Resources/pathway"
  codesign --force --sign "$IDENTITY" "$APP_BUNDLE/Contents/MacOS/PathwayShim"
  codesign --force --sign "$IDENTITY" "$APP_BUNDLE"
fi

codesign --verify --strict --verbose=2 "$APP_BUNDLE"

OUTPUT_ZIP="$DIST_DIR/Pathway-${VERSION}.zip"
rm -f "$OUTPUT_ZIP"
ditto -c -k --keepParent "$APP_BUNDLE" "$OUTPUT_ZIP"

echo "Created bundle at $APP_BUNDLE"
echo "Created archive at $OUTPUT_ZIP"
