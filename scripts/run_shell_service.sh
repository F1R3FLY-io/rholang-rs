#!/usr/bin/env bash
# Run rholang-shell (rhosh) as a TCP service exposed on a given port (default: 8666).
# Each incoming TCP connection gets its own shell session.
#
# Requirements:
#   - socat (for TCP <-> PTY bridging so line-editing works)
#   - cargo (to build rhosh if needed)
#
# Usage:
#   ./scripts/run_shell_service.sh            # listen on 0.0.0.0:8666
#   ./scripts/run_shell_service.sh --port 7777
#   PORT=9000 ./scripts/run_shell_service.sh
#
# Connect with:
#   nc 127.0.0.1 8666
# or
#   telnet 127.0.0.1 8666

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PORT="${PORT:-}"
if [[ -z "${PORT}" ]]; then
  PORT=8666
fi

if [[ "${1-}" == "--port" && -n "${2-}" ]]; then
  PORT="$2"
fi

# Check dependencies
if ! command -v socat >/dev/null 2>&1; then
  echo "Error: 'socat' is required but not installed." >&2
  echo "Install it and retry. Examples:" >&2
  echo "  macOS (brew):   brew install socat" >&2
  echo "  Debian/Ubuntu:  sudo apt-get update && sudo apt-get install -y socat" >&2
  echo "  Alpine:         apk add socat" >&2
  exit 1
fi

# Ensure the shell binary exists
BIN_PATH="target/release/rhosh"
if [[ ! -x "$BIN_PATH" ]]; then
  echo "Building rholang-shell (release)..."
  cargo build -p rholang-shell --release
fi

# Helpful info
echo "Starting rholang-shell service on 0.0.0.0:${PORT}"
echo "Each new TCP connection spawns an isolated shell session."
echo "Binary: $BIN_PATH"

# Clean shutdown handler
cleanup() {
  echo
  echo "Shutting down rholang-shell service..."
}
trap cleanup EXIT INT TERM

#
# socat notes:
# - fork: handle multiple sequential connections (one process per connection)
# - reuseaddr: allow quick restarts
# - pty,setsid,ctty: allocate a pseudo-TTY for rhosh so line editing works
# - raw,echo=0: cleaner TTY behavior for interactive shell
#
# We also pass -m (multiline) if desired; keep default single-line for network usage.
#

exec socat -d -d TCP-LISTEN:${PORT},reuseaddr,fork "EXEC:'${BIN_PATH}',pty,setsid,ctty,raw,echo=0"