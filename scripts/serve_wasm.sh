#!/usr/bin/env bash
set -euo pipefail

# Serve the rholang-wasm example site after building the WASM package.
#
# Usage:
#   scripts/serve_wasm.sh [--port <port>] [--release] [--open] [--features <features>]
#
# Options:
#   --port <port>   Port to serve on (default: 8000)
#   --release       Build with optimizations (wasm-pack --release)
#   --open          Open the demo page in your default browser after starting the server
#   --features <features>  Cargo features to enable (comma-separated). Default: vm-eval
#
# Prerequisites:
#   - Rust toolchain
#   - wasm-pack (install with: cargo install wasm-pack)
#   - Python 3 (for a simple static HTTP server)
#
# The server serves the rholang-wasm directory so the demo is at:
#   http://localhost:<port>/index.html

PORT=8000
RELEASE=0
OPEN_BROWSER=0
FEATURES="vm-eval"

print_usage() {
  sed -n '1,60p' "$0" | sed 's/^# \{0,1\}//'
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT=${2:-}
      if [[ -z "$PORT" ]]; then
        echo "error: --port requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    --release)
      RELEASE=1
      shift
      ;;
    --open)
      OPEN_BROWSER=1
      shift
      ;;
    --features)
      FEATURES=${2:-}
      if [[ -z "$FEATURES" && -z "${2+x}" ]]; then
        echo "error: --features requires a value (use --features \"\" to disable)" >&2
        exit 1
      fi
      shift 2
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1" >&2
      print_usage
      exit 1
      ;;
  esac
done

# Ensure required tools exist
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack is not installed." >&2
  echo "Install it with: cargo install wasm-pack" >&2
  exit 1
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "error: python3 is required to run the simple web server." >&2
  exit 1
fi

# Determine repo root (directory of this script is <repo>/scripts)
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
WASM_DIR="$REPO_ROOT/rholang-wasm"

if [[ ! -d "$WASM_DIR" ]]; then
  echo "error: Could not find rholang-wasm directory at: $WASM_DIR" >&2
  exit 1
fi

# Build the WASM package
cd "$WASM_DIR"
BUILD_FLAGS=(build --target web)
if [[ "$RELEASE" -eq 1 ]]; then
  BUILD_FLAGS+=(--release)
fi
if [[ -n "$FEATURES" ]]; then
  BUILD_FLAGS+=(--features "$FEATURES")
fi

echo "[rholang-wasm] Running: wasm-pack ${BUILD_FLAGS[*]}"
if ! wasm-pack "${BUILD_FLAGS[@]}"; then
  if [[ -n "$FEATURES" ]]; then
    echo "[rholang-wasm] warning: wasm build failed with features: '$FEATURES'. Falling back to no features." >&2
    BUILD_FLAGS=(build --target web)
    if [[ "$RELEASE" -eq 1 ]]; then BUILD_FLAGS+=(--release); fi
    echo "[rholang-wasm] Running (fallback): wasm-pack ${BUILD_FLAGS[*]}"
    wasm-pack "${BUILD_FLAGS[@]}"
  else
    exit 1
  fi
fi

# Start the static server from rholang-wasm directory so index.html is at /
cd "$WASM_DIR"
URL="http://localhost:${PORT}/index.html"

echo "[rholang-wasm] Serving $WASM_DIR on port $PORT"
echo "[rholang-wasm] Open: $URL"

# Optionally open the browser after a short delay
if [[ "$OPEN_BROWSER" -eq 1 ]]; then
  (
    sleep 1
    if command -v xdg-open >/dev/null 2>&1; then
      xdg-open "$URL" >/dev/null 2>&1 || true
    elif command -v open >/dev/null 2>&1; then
      open "$URL" >/dev/null 2>&1 || true
    elif command -v start >/dev/null 2>&1; then
      start "$URL" >/dev/null 2>&1 || true
    else
      echo "[rholang-wasm] Could not detect a browser opener; please open $URL manually." >&2
    fi
  ) &
fi

# Run python server (foreground)
exec python3 -m http.server "$PORT" --bind 127.0.0.1
