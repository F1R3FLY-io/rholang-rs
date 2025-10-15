#!/usr/bin/env bash
# Serve the rholang-shell WASM demo webpage locally
#
# This script builds the wasm package (if not present) and starts a static server
# to host rholang-shell/www and rholang-shell/pkg, then opens the demo page.
#
# Usage:
#   ./scripts/serve_wasm_shell.sh [--port PORT] [--dev|--release]
#
# Examples:
#   ./scripts/serve_wasm_shell.sh
#   ./scripts/serve_wasm_shell.sh --port 9000 --dev

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PORT=8080
PROFILE_FLAG="--release"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --port)
      PORT="${2:-8080}"
      shift 2
      ;;
    --dev)
      PROFILE_FLAG="--dev"
      shift
      ;;
    --release)
      PROFILE_FLAG="--release"
      shift
      ;;
    *)
      echo "Unknown option: $1" >&2
      shift
      ;;
  esac
done

# Ensure a wasm build exists
if [ ! -d "rholang-shell/pkg" ]; then
  echo "No wasm pkg found. Building first..."
  "$SCRIPT_DIR/build_wasm_shell.sh" "$PROFILE_FLAG"
fi

# Ensure demo exists
if [ ! -f "rholang-shell/www/index.html" ]; then
  echo "Error: demo page not found at rholang-shell/www/index.html"
  exit 1
fi

# Start a simple static server rooted at rholang-shell so ../pkg is reachable
cd rholang-shell

URL="http://127.0.0.1:$PORT/www/index.html"

# Try to open browser in background (best-effort)
open_browser() {
  if command -v xdg-open >/dev/null 2>&1; then
    xdg-open "$URL" >/dev/null 2>&1 || true
  elif command -v open >/dev/null 2>&1; then
    open "$URL" >/dev/null 2>&1 || true
  elif command -v start >/dev/null 2>&1; then
    start "$URL" >/dev/null 2>&1 || true
  fi
}

# Prefer Python 3 http.server
if command -v python3 >/dev/null 2>&1; then
  echo "Serving rholang-shell at http://127.0.0.1:$PORT/"
  echo "Opening $URL"
  open_browser || true
  exec python3 -m http.server "$PORT"
elif command -v python >/dev/null 2>&1; then
  echo "Serving rholang-shell at http://127.0.0.1:$PORT/"
  echo "Opening $URL"
  open_browser || true
  exec python -m SimpleHTTPServer "$PORT"
else
  echo "No Python found. Please serve 'rholang-shell' directory with any static file server and open: $URL"
  exit 1
fi
