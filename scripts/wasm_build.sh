#!/usr/bin/env bash
set -euo pipefail

# wasm_build.sh
# Builds the rholang-wasm package and the React (Vite) app for production.
#
# Usage:
#   ./scripts/wasm_build.sh [--debug]
#
# Options:
#   --debug   Build WASM in debug mode (default: release/optimized)
#
# Output:
#   - rholang-wasm/pkg/* (wasm-bindgen JS glue + .wasm)
#   - rholang-wasm/www/dist (production web assets)

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

PROFILE=release
for arg in "$@"; do
  case "$arg" in
    --debug) PROFILE=dev ;;
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
if ! command -v rustup >/dev/null 2>&1; then
  echo "[error] rustup not found. Please install rustup." >&2
  exit 1
fi

export RUSTUP_TOOLCHAIN=nightly

# Ensure wasm32 target is available
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

echo "[info] Building React app (Vite) for production ..."
pushd www >/dev/null
if [[ ! -d node_modules ]]; then
  echo "[info] Installing npm dependencies ..."
  npm install
fi
npm run build
popd >/dev/null
popd >/dev/null

echo "[done] Build complete. Production assets are in: rholang-wasm/www/dist"
echo "       Preview locally with: (cd rholang-wasm/www && npm run preview)"
