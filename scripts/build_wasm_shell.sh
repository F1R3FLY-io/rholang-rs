#!/usr/bin/env bash
# Build the rholang-shell crate for the Web (WASM) target using wasm-pack
#
# Usage:
#   ./scripts/build_wasm_shell.sh [--dev|--release] [--no-install]
#
# Examples:
#   ./scripts/build_wasm_shell.sh                  # default: release build
#   ./scripts/build_wasm_shell.sh --dev            # dev build (faster, larger)
#   ./scripts/build_wasm_shell.sh --release        # explicit release build
#   ./scripts/build_wasm_shell.sh --no-install     # skip installing wasm-pack if missing

set -euo pipefail

# Move to repository root (script may be called from anywhere)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Load helper functions if available
if [ -f "scripts/common.sh" ]; then
  # shellcheck source=./common.sh
  . "scripts/common.sh"
fi

# Ensure we are at project root
if [ -f "scripts/common.sh" ]; then
  check_project_root
fi

BUILD_PROFILE="--release"
SKIP_INSTALL=0

for arg in "$@"; do
  case "$arg" in
    --dev)
      BUILD_PROFILE="--dev"
      shift
      ;;
    --release)
      BUILD_PROFILE="--release"
      shift
      ;;
    --no-install)
      SKIP_INSTALL=1
      shift
      ;;
    *)
      ;;
  esac
done

# Check wasm-pack is installed
if ! command -v wasm-pack >/dev/null 2>&1; then
  if [ "$SKIP_INSTALL" -eq 1 ]; then
    echo "Error: wasm-pack not found. Install with: cargo install wasm-pack"
    exit 1
  fi
  echo "wasm-pack not found. Installing (cargo install wasm-pack@0.13.1)..."
  cargo install wasm-pack@0.13.1
fi

# Ensure a matching wasm-bindgen-cli is available to avoid version skew
if ! command -v wasm-bindgen >/dev/null 2>&1; then
  if [ "$SKIP_INSTALL" -eq 1 ]; then
    echo "Error: wasm-bindgen-cli not found. Install with: cargo install wasm-bindgen-cli --version 0.2.104"
    exit 1
  fi
  echo "Installing wasm-bindgen-cli 0.2.104..."
  cargo install wasm-bindgen-cli --version 0.2.104
fi

# Build the rholang-shell crate for the web target
cd rholang-shell

echo "Building rholang-shell for WebAssembly (target=web, features=wasm) with wasm-pack..."
# Note: wasm-pack uses package name -> JS file `rholang_shell.js` (hyphens -> underscores)
# The output will go to ./pkg
# Ensure host-specific RUSTFLAGS (like -C target-cpu=native/apple-m3) don't break wasm cross-compiles
ORIG_RUSTFLAGS="${RUSTFLAGS-}"
ORIG_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS-}"
unset RUSTFLAGS
unset CARGO_ENCODED_RUSTFLAGS
# wasm-pack v0.13 enforces release profile internally and rejects extra --dev/--release flags.
# Invoke without profile flags for compatibility.
wasm-pack build --target web --no-default-features --features wasm
# Restore environment
export RUSTFLAGS="${ORIG_RUSTFLAGS}"
export CARGO_ENCODED_RUSTFLAGS="${ORIG_ENCODED_RUSTFLAGS}"

echo "✅ Build complete. Output available in rholang-shell/pkg"

# Create demo directory if missing and place a README note
mkdir -p www
if [ ! -f www/README.txt ]; then
  cat > www/README.txt <<'EOF'
This directory contains a minimal demo webpage for the rholang-shell WebAssembly build.
Use ../../scripts/serve_wasm_shell.sh to start a local server and open the demo.
EOF
fi

