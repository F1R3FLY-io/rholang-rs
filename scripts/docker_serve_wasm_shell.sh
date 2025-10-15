#!/usr/bin/env bash
# Build and run the rholang-shell WASM demo in Docker, then test endpoints.
# Usage:
#   ./scripts/docker_serve_wasm_shell.sh [--port 8080]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PORT=8080
if [[ "${1-}" == "--port" && -n "${2-}" ]]; then
  PORT="$2"
fi

IMAGE_NAME="rholang-wasm"

echo "Building Docker image ($IMAGE_NAME)..."
docker build -f Dockerfile.wasm -t "$IMAGE_NAME" .

echo "Running container on port $PORT..."
CID=$(docker run -d -p "${PORT}:8080" "$IMAGE_NAME")
trap 'echo "Stopping container..."; docker rm -f "$CID" >/dev/null 2>&1 || true' EXIT

# Basic readiness wait
ATTEMPTS=30
until curl -fsS "http://127.0.0.1:${PORT}/www/index.html" >/dev/null 2>&1; do
  ATTEMPTS=$((ATTEMPTS-1))
  if [[ $ATTEMPTS -le 0 ]]; then
    echo "Server did not become ready in time" >&2
    exit 1
  fi
  sleep 0.5
done

echo "✅ index.html reachable"

# Verify the JS glue is served (path relative to /www -> ../pkg)
if curl -fsSI "http://127.0.0.1:${PORT}/pkg/rholang_wasm.js" | grep -qi '200 OK'; then
  echo "✅ rholang_wasm.js reachable"
else
  echo "❌ Failed to reach rholang_wasm.js" >&2
  exit 1
fi

# Test API endpoint with a simple snippet
echo "Testing /api/run..."
API_RESP=$(curl -fsS -X POST "http://127.0.0.1:${PORT}/api/run" -H 'Content-Type: application/json' --data '{"code":"1 + 2"}') || true
if echo "$API_RESP" | grep -q '"ok":true'; then
  echo "✅ /api/run responded with ok:true"
else
  echo "❌ /api/run did not respond with ok:true: $API_RESP" >&2
  exit 1
fi

# Print the URL for user convenience
echo "Open: http://127.0.0.1:${PORT}/www/index.html"

# Keep running and show logs until interrupted
echo "Press Ctrl+C to stop. Tailing container logs..."
docker logs -f "$CID"
