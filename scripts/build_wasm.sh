#!/usr/bin/env bash
set -euo pipefail

# Build the rholang-wasm package with wasm-pack.
#
# Usage:
#   scripts/build_wasm.sh [--release] [--target <web|bundler|nodejs|no-modules>] [--out-dir <dir>] [--features <features>] [--] [EXTRA_WASM_PACK_ARGS...]
#
# Options:
#   --release                 Build with optimizations
#   --target <tgt>            wasm-pack target (default: web)
#   --out-dir <dir>           Output directory for generated pkg (default: pkg)
#   --features <features>     Cargo features to enable (comma-separated). Default: vm-eval
#   -h, --help                Show this help message
#
# Notes:
# - By default, this script enables the `vm-eval` feature for WASM builds.
#   To disable or override, pass `--features ""` (disable) or another list.
# - You can pass additional arguments to wasm-pack after a literal `--`.
# - Prerequisite: `wasm-pack` must be installed (install with: cargo install wasm-pack).

TARGET="web"
RELEASE=0
OUT_DIR="pkg"
FEATURES="vm-eval"

print_usage() {
  sed -n '1,200p' "$0" | sed 's/^# \{0,1\}//'
}

# Parse arguments
EXTRA_ARGS=()
while [[ $# -gt 0 ]]; do
  case "$1" in
    --release)
      RELEASE=1
      shift
      ;;
    --target)
      TARGET=${2:-}
      if [[ -z "$TARGET" ]]; then
        echo "error: --target requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    --out-dir)
      OUT_DIR=${2:-}
      if [[ -z "$OUT_DIR" ]]; then
        echo "error: --out-dir requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    --features)
      FEATURES=${2:-}
      if [[ -z "$FEATURES" ]]; then
        echo "error: --features requires a value" >&2
        exit 1
      fi
      shift 2
      ;;
    -h|--help)
      print_usage
      exit 0
      ;;
    --)
      shift
      EXTRA_ARGS=("$@")
      break
      ;;
    *)
      echo "Unknown option: $1" >&2
      print_usage
      exit 1
      ;;
  esac
done

# Ensure tools
if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "error: wasm-pack is not installed." >&2
  echo "Install it with: cargo install wasm-pack" >&2
  exit 1
fi

# Resolve directories
SCRIPT_DIR=$(cd "$(dirname "$0")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
WASM_DIR="$REPO_ROOT/rholang-wasm"

if [[ ! -d "$WASM_DIR" ]]; then
  echo "error: Could not find rholang-wasm directory at: $WASM_DIR" >&2
  exit 1
fi

cd "$WASM_DIR"

# Build command
CMD=(wasm-pack build --target "$TARGET" --out-dir "$OUT_DIR")
if [[ $RELEASE -eq 1 ]]; then
  CMD+=(--release)
fi
if [[ -n "$FEATURES" ]]; then
  CMD+=("--features" "$FEATURES")
fi
if [[ ${#EXTRA_ARGS[@]} -gt 0 ]]; then
  CMD+=("${EXTRA_ARGS[@]}")
fi

# Echo and run with fallback if vm-eval causes a wasm32 incompatibility
if [[ -n "$FEATURES" ]]; then
  echo "[rholang-wasm] Running: ${CMD[*]}"
  if ! "${CMD[@]}"; then
    echo "[rholang-wasm] warning: wasm build failed with features: '$FEATURES'." >&2
    echo "[rholang-wasm] hint   : Retrying without features to keep the demo build working. Use --features to force." >&2
    # Retry without features
    CMD_NOFEAT=(wasm-pack build --target "$TARGET" --out-dir "$OUT_DIR")
    if [[ $RELEASE -eq 1 ]]; then CMD_NOFEAT+=(--release); fi
    echo "[rholang-wasm] Running (fallback): ${CMD_NOFEAT[*]}"
    "${CMD_NOFEAT[@]}"
  fi
else
  echo "[rholang-wasm] Running: ${CMD[*]}"
  "${CMD[@]}"
fi

# Output summary
ABS_OUT_DIR=$(cd "$OUT_DIR" 2>/dev/null && pwd || echo "$OUT_DIR")
echo ""
echo "[rholang-wasm] Build complete."
echo "  Target     : $TARGET"
echo "  Profile    : $([[ $RELEASE -eq 1 ]] && echo release || echo debug)"
echo "  Output dir : $ABS_OUT_DIR"
echo ""
echo "Artifacts:"
echo "  - JS shim : $OUT_DIR/rholang_wasm.js"
echo "  - WASM    : $OUT_DIR/rholang_wasm_bg.wasm"
echo ""
echo "Next steps:"
echo "  - To serve the demo page: scripts/serve_wasm.sh --port 8000${RELEASE:+ --release}"
echo "  - Or open rholang-wasm/index.html with a static server that supports ES modules."
