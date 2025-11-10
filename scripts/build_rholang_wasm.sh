#!/usr/bin/env bash
set -euo pipefail

# build_rholang_wasm.sh
# Builds the rholang-wasm web package. Supports two modes:
# 1) Native wasm-pack build serving static files (default)
# 2) Docker image build (nginx serving built assets)
#
# Usage:
#   ./scripts/build_rholang_wasm.sh [--docker] [--release]
#
# Options:
#   --docker    Build a Docker image named rholang-wasm:latest using rholang-wasm-draft/Dockerfile
#   --release   Build with optimizations (for native wasm-pack mode)
#
# Prereqs (native): rustup toolchain, wasm-pack (cargo install wasm-pack)
# Prereqs (docker): Docker installed and running

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
cd "$ROOT_DIR"

MODE="native"
PROFILE="dev"

for arg in "$@"; do
  case "$arg" in
    --docker)
      MODE="docker" ;;
    --release)
      PROFILE="release" ;;
    -h|--help)
      grep -E "^#( |$)" "$0" | sed 's/^# //; s/^#//' ; exit 0 ;;
    *)
      echo "Unknown option: $arg" >&2; exit 2 ;;
  esac
done

if [[ "$MODE" == "docker" ]]; then
  echo "[build] Building Docker image rholang-wasm:latest ..."
  docker build -t rholang-wasm -f rholang-wasm-draft/Dockerfile .
  echo "[build] Docker image built: rholang-wasm:latest"
  exit 0
fi

# Native wasm-pack build
# Ensure nightly toolchain is used
export RUSTUP_TOOLCHAIN=nightly

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "wasm-pack not found. Install with: cargo install wasm-pack" >&2
  exit 1
fi

pushd rholang-wasm >/dev/null

# Clean previous build output to ensure fresh artifacts
rm -rf pkg

if [[ "$PROFILE" == "release" ]]; then
  echo "[build] wasm-pack build --target web --out-dir pkg --release"
  wasm-pack build --target web --out-dir pkg --release
else
  echo "[build] wasm-pack build --target web --out-dir pkg"
  wasm-pack build --target web --out-dir pkg
fi

# Ensure there is an index.html available for the dev app if missing
if [[ ! -f www/index.html ]]; then
  mkdir -p www
  cat > www/index.html <<'HTML'
<!doctype html>
<html>
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Rholang WASM Test</title>
  <style>
    body { font-family: system-ui, -apple-system, Segoe UI, Roboto, sans-serif; margin: 2rem; }
    textarea { width: 100%; height: 200px; font-family: monospace; }
    #output { white-space: pre-wrap; background: #111; color: #0f0; padding: 1rem; border-radius: 6px; min-height: 120px; }
    .row { display: grid; grid-template-columns: 1fr; gap: 1rem; }
    button { padding: .6rem 1rem; }
  </style>
</head>
<body>
  <h1>Rholang WASM Tester</h1>
  <div class="row">
    <label for="code">Rholang code</label>
    <textarea id="code" placeholder="// Type Rholang here"></textarea>
    <div>
      <button id="run">Run</button>
      <span id="status"></span>
    </div>
    <div id="output" aria-live="polite"></div>
  </div>
  <script type="module">
    import init, { WasmInterpreter } from './pkg/rholang_wasm.js';

    const status = document.getElementById('status');
    const output = document.getElementById('output');
    const code = document.getElementById('code');
    const runBtn = document.getElementById('run');

    async function main() {
      status.textContent = 'Loading...';
      await init();
      status.textContent = 'Ready';
      const interp = new WasmInterpreter();
      runBtn.onclick = async () => {
        status.textContent = 'Running...';
        try {
          const res = await interp.interpret(code.value);
          output.textContent = res;
          status.textContent = 'Done';
        } catch (e) {
          output.textContent = 'Error: ' + e;
          status.textContent = 'Error';
        }
      };
    }

    main();
  </script>
</body>
</html>
HTML
fi

popd >/dev/null

echo "[build] rholang-wasm assets built at rholang-wasm/pkg"
