#!/usr/bin/env bash
set -euo pipefail

# run_rholang_wasm.sh
# Runs the rholang-wasm testing page locally.
# Supports two modes:
# 1) Native static server serving rholang-wasm-draft/www on a configurable port
# 2) Docker container (nginx) from image rholang-wasm:latest
#
# Usage:
#   ./scripts/run_rholang_wasm.sh [--docker] [--port PORT]
#
# Options:
#   --docker       Run via Docker image rholang-wasm:latest
#   --port PORT    Port to serve on (default: 8080 for native, 8080->80 for docker)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

MODE="native"
PORT="8080"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --docker)
      MODE="docker"; shift ;;
    --port)
      PORT="${2:-}"; shift 2 ;;
    -h|--help)
      grep -E "^#( |$)" "$0" | sed 's/^# //; s/^#//' ; exit 0 ;;
    *)
      echo "Unknown option: $1" >&2; exit 2 ;;
  esac
done

if [[ "$MODE" == "docker" ]]; then
  # Ensure image exists
  if ! docker image inspect rholang-wasm:latest >/dev/null 2>&1; then
    echo "[run] Docker image rholang-wasm:latest not found. Building it..."
    ./scripts/build_rholang_wasm.sh --docker
  fi
  echo "[run] Starting Docker container on http://localhost:$PORT ... (Ctrl+C to stop)"
  exec docker run --rm --name rholang-wasm \
    -p "$PORT:80" \
    rholang-wasm:latest
fi

# Native mode: build assets if needed and serve from rholang-wasm-draft/www
if [[ ! -d rholang-wasm-draft/www/pkg ]]; then
  echo "[run] No built assets found. Building..."
  ./scripts/build_rholang_wasm.sh
fi

SERVE_DIR="rholang-wasm-draft/www"
if [[ ! -d "$SERVE_DIR" ]]; then
  echo "[run] Serve directory $SERVE_DIR not found" >&2
  exit 1
fi

echo "[run] Serving $SERVE_DIR at http://localhost:$PORT"

# Prefer basic http servers available on most Linux distros
if command -v python3 >/dev/null 2>&1; then
  cd "$SERVE_DIR"
  exec python3 -m http.server "$PORT"
elif command -v busybox >/dev/null 2>&1; then
  exec busybox httpd -f -p ":$PORT" -h "$SERVE_DIR"
elif command -v python >/dev/null 2>&1; then
  cd "$SERVE_DIR"
  exec python -m SimpleHTTPServer "$PORT"
else
  echo "No simple HTTP server found (python3/busybox/python). Please install one or use --docker." >&2
  exit 1
fi
