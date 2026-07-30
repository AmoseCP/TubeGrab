#!/usr/bin/env bash
# Downloads yt-dlp and ffmpeg (macOS) into src-tauri/binaries/.
# Usage: scripts/fetch-binaries.sh [arm64|x64]   (defaults to host arch)
set -euo pipefail

ARCH="${1:-}"
if [ -z "$ARCH" ]; then
  case "$(uname -m)" in
    arm64) ARCH=arm64 ;;
    *) ARCH=x64 ;;
  esac
fi

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$BIN_DIR"

echo "Downloading yt-dlp (macOS universal) ..."
curl -fL -o "$BIN_DIR/yt-dlp" \
  "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"

echo "Downloading ffmpeg (darwin-$ARCH) ..."
curl -fL -o "$BIN_DIR/ffmpeg" \
  "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/ffmpeg-darwin-$ARCH"

chmod +x "$BIN_DIR/yt-dlp" "$BIN_DIR/ffmpeg"
echo "Done:"
ls -lh "$BIN_DIR"
