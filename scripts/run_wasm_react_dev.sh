#!/usr/bin/env bash
set -euo pipefail

# run_wasm_react_dev.sh
# Build the rholang-wasm package and start the React (Vite) dev server.
#
# Usage:
#   ./scripts/run_wasm_react_dev.sh [--release]
#
# Options:
#   --release  Build WASM in release mode (optimized). Default is debug.
#
# Prerequisites:
#   - rustup + cargo
#   - wasm-pack (install: cargo install wasm-pack)
#   - Node.js 18+ and npm

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

PROFILE=dev
for arg in "$@"; do
  case "$arg" in
    --release) PROFILE=release ;;
    -h|--help)
      grep -E "^#( |$)" "$0" | sed 's/^# //; s/^#//' ; exit 0 ;;
    *)
      echo "Unknown option: $arg" >&2 ; exit 2 ;;
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

echo "[info] Starting React dev server (Vite) ..."
pushd www >/dev/null

# Install dependencies if node_modules missing
if [[ ! -d node_modules ]]; then
  echo "[info] Installing npm dependencies ..."
  npm install
fi

# Use the package script that rebuilds WASM then starts Vite for convenience
npm run dev:wasm

popd >/dev/null
popd >/dev/null
