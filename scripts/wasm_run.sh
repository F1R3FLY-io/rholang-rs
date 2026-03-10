#!/usr/bin/env bash
set -euo pipefail

# wasm_run.sh
# Builds the rholang-wasm package (debug by default) and starts the React (Vite) dev server.
#
# Usage:
#   ./scripts/wasm_run.sh [--release] [--port <PORT>] [--host <HOST>]
#
# Options:
#   --release       Build WASM in release mode (optimized). Default is debug.
#   --port <PORT>   Port for Vite dev server (default: 5173)
#   --host <HOST>   Host for Vite dev server (default: 127.0.0.1)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

PROFILE=dev
PORT=5173
HOST=127.0.0.1
while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      PROFILE=release
      shift
      ;;
    --port)
      shift
      PORT=${1:-5173}
      shift
      ;;
    --host)
      shift
      HOST=${1:-127.0.0.1}
      shift
      ;;
    -h|--help)
      grep -E "^#( |$)" "$0" | sed 's/^# //; s/^#//' ; exit 0 ;;
    *)
      echo "Unknown option: $1" >&2 ; exit 2 ;;
  esac
done

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "[error] wasm-pack not found. Install with: cargo install wasm-pack" >&2
  exit 1
fi
if ! command -v npm >/dev/null 2>&1; then
  echo "[error] npm not found. Please install Node.js (18+) and npm." >&2
  exit 1
fi
if ! command -v rustup >/dev/null 2>&1; then
  echo "[error] rustup not found. Please install rustup." >&2
  exit 1
fi

export RUSTUP_TOOLCHAIN=nightly

# Ensure wasm32 target is installed for Rust toolchain
if ! rustup target list --installed | grep -q "wasm32-unknown-unknown"; then
  echo "[info] Adding Rust target wasm32-unknown-unknown ..."
  rustup target add wasm32-unknown-unknown
fi

echo "[info] Building rholang-wasm package ($PROFILE) ..."
pushd rholang-wasm >/dev/null
rm -rf pkg
if [[ "$PROFILE" == "release" ]]; then
  wasm-pack build --target web --out-dir pkg --release
else
  wasm-pack build --target web --out-dir pkg
fi

echo "[info] Starting React dev server (Vite) on $HOST:$PORT ..."
pushd www >/dev/null

# Install dependencies if node_modules missing
if [[ ! -d node_modules ]]; then
  echo "[info] Installing npm dependencies ..."
  npm install
fi

# Use the package script that rebuilds WASM then starts Vite for convenience
npm run dev:wasm -- --host "$HOST" --port "$PORT"

popd >/dev/null
popd >/dev/null
