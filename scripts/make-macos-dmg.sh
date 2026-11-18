#!/usr/bin/env bash
# Package an already-built .app without mounting a disk image or automating Finder.
# Usage: bash scripts/make-macos-dmg.sh [aarch64-apple-darwin|x86_64-apple-darwin]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TARGET="${1:-}"
case "$TARGET" in
  aarch64-apple-darwin) ARCH=aarch64 ;;
  x86_64-apple-darwin) ARCH=x64 ;;
  "")
    case "$(uname -m)" in
      arm64) ARCH=aarch64 ;;
      x86_64) ARCH=x64 ;;
      *) echo "This script requires macOS." >&2; exit 1 ;;
    esac ;;
  *) echo "Unsupported target: $TARGET" >&2; exit 1 ;;
esac

BUNDLE_DIR="$ROOT/src-tauri/target/${TARGET:+$TARGET/}release/bundle"
APP="$BUNDLE_DIR/macos/TubeGrab.app"
if [ ! -d "$APP" ]; then
  echo "Build the app first: npm run tauri build -- --bundles app" >&2
  exit 1
fi
VERSION="$(node -p "JSON.parse(require('fs').readFileSync(process.argv[1], 'utf8')).version" "$ROOT/package.json")"
mkdir -p "$BUNDLE_DIR/dmg"
STAGING="$(mktemp -d "$BUNDLE_DIR/dmg/staging.XXXXXX")"
trap 'rm -rf "$STAGING"' EXIT
ditto "$APP" "$STAGING/TubeGrab.app"
ln -s /Applications "$STAGING/Applications"
DMG="$BUNDLE_DIR/dmg/TubeGrab_${VERSION}_${ARCH}.dmg"
hdiutil create -volname TubeGrab -srcfolder "$STAGING" -ov -format UDZO "$DMG"
hdiutil verify "$DMG"
echo "Installer: $DMG"
