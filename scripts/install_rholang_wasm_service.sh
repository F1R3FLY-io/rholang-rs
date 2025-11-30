#!/usr/bin/env bash
set -euo pipefail

# install_rholang_wasm_service.sh
# Installs and manages a systemd service to serve the rholang-wasm testing page via Docker.
# This script must be run with sudo/root privileges to install to /etc.
#
# Usage:
#   sudo ./scripts/install_rholang_wasm_service.sh install [--port 8080] [--image rholang-wasm:latest]
#   sudo ./scripts/install_rholang_wasm_service.sh uninstall
#   sudo ./scripts/install_rholang_wasm_service.sh start|stop|restart|status|logs
#
# Notes:
# - The service uses /etc/default/rholang-wasm for configuration (PORT, IMAGE)
# - By default, PORT=8080 and IMAGE=rholang-wasm:latest
# - Build the image beforehand with: ./scripts/build_rholang_wasm.sh --docker
#

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
SERVICE_NAME="rholang-wasm"
UNIT_SRC="$ROOT_DIR/scripts/rholang-wasm.service"
UNIT_DST="/etc/systemd/system/${SERVICE_NAME}.service"
ENV_FILE="/etc/default/${SERVICE_NAME}"

require_root() {
  if [[ "${EUID:-$(id -u)}" -ne 0 ]]; then
    echo "This action requires root. Please run with sudo." >&2
    exit 1
  fi
}

install_service() {
  local port="${1:-8080}" image="${2:-rholang-wasm:latest}"
  require_root
  echo "[install] Installing ${SERVICE_NAME} service (port=${port}, image=${image})"

  if [[ ! -f "$UNIT_SRC" ]]; then
    echo "Unit file not found at $UNIT_SRC" >&2
    exit 1
  fi

  # Copy unit
  install -D -m 0644 "$UNIT_SRC" "$UNIT_DST"

  # Write env file
  mkdir -p "$(dirname "$ENV_FILE")"
  cat > "$ENV_FILE" <<EOF
PORT=${port}
IMAGE=${image}
EOF
  chmod 0644 "$ENV_FILE"

  # Reload and enable
  systemctl daemon-reload
  systemctl enable --now "$SERVICE_NAME"

  echo "[install] Service installed and started. Visit: http://<host>:${port}"
}

uninstall_service() {
  require_root
  echo "[uninstall] Removing ${SERVICE_NAME} service"
  systemctl disable --now "$SERVICE_NAME" || true
  rm -f "$UNIT_DST"
  rm -f "$ENV_FILE"
  systemctl daemon-reload
  echo "[uninstall] Done"
}

cmd=${1:-}
case "$cmd" in
  install)
    shift || true
    PORT="8080"
    IMAGE="rholang-wasm:latest"
    while [[ $# -gt 0 ]]; do
      case "$1" in
        --port) PORT="${2:-}"; shift 2 ;;
        --image) IMAGE="${2:-}"; shift 2 ;;
        *) echo "Unknown option: $1" >&2; exit 2 ;;
      esac
    done
    install_service "$PORT" "$IMAGE" ;;
  uninstall)
    uninstall_service ;;
  start|stop|restart|status)
    require_root
    systemctl "$cmd" "$SERVICE_NAME" ;;
  logs)
    journalctl -u "$SERVICE_NAME" -e -f ;;
  -h|--help|help|*)
    grep -E "^#( |$)" "$0" | sed 's/^# //; s/^#//' ;;
 esac
