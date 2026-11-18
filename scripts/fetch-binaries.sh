#!/usr/bin/env bash
# Downloads the self-contained macOS download engine into src-tauri/binaries/.
# Usage: scripts/fetch-binaries.sh [arm64|x64]   (defaults to host arch)
set -euo pipefail

ARCH="${1:-}"
if [ -z "$ARCH" ]; then
  case "$(uname -m)" in
    arm64) ARCH=arm64 ;;
    *) ARCH=x64 ;;
  esac
fi
case "$ARCH" in
  arm64) DENO_ARCH=aarch64 ;;
  x64) DENO_ARCH=x86_64 ;;
  *) echo "Usage: $0 [arm64|x64]" >&2; exit 1 ;;
esac

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN_DIR="$ROOT/src-tauri/binaries"
mkdir -p "$BIN_DIR"

echo "Downloading yt-dlp (macOS universal) ..."
curl -fL -o "$BIN_DIR/yt-dlp" \
  "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_macos"

for tool in ffmpeg ffprobe; do
  echo "Downloading $tool (darwin-$ARCH) ..."
  curl -fL --retry 3 -o "$BIN_DIR/$tool" \
    "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/$tool-darwin-$ARCH"
done

# YouTube's challenge solver requires a JavaScript runtime even with standalone yt-dlp.
echo "Downloading Deno ($DENO_ARCH) ..."
DENO_ZIP="$(mktemp -t tubegrab-deno).zip"
trap 'rm -f "$DENO_ZIP" "${DENO_ZIP%.zip}"' EXIT
curl -fL --retry 3 -o "$DENO_ZIP" \
  "https://github.com/denoland/deno/releases/latest/download/deno-$DENO_ARCH-apple-darwin.zip"
unzip -oq "$DENO_ZIP" deno -d "$BIN_DIR"

mkdir -p "$BIN_DIR/licenses"
curl -fL --retry 3 -o "$BIN_DIR/licenses/ffmpeg-LICENSE" \
  "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/darwin-$ARCH.LICENSE"
curl -fL --retry 3 -o "$BIN_DIR/licenses/ffmpeg-README" \
  "https://github.com/eugeneware/ffmpeg-static/releases/latest/download/darwin-$ARCH.README"
curl -fL --retry 3 -o "$BIN_DIR/licenses/deno-LICENSE" \
  "https://raw.githubusercontent.com/denoland/deno/main/LICENSE.md"

chmod +x "$BIN_DIR/yt-dlp" "$BIN_DIR/ffmpeg" "$BIN_DIR/ffprobe" "$BIN_DIR/deno"
echo "Done:"
ls -lh "$BIN_DIR"
