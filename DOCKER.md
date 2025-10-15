# Rholang Development Container

This document provides instructions for using the development container for the Rholang project.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/)
- [Docker Compose](https://docs.docker.com/compose/install/)
- [Visual Studio Code](https://code.visualstudio.com/) (optional, for VS Code integration)
- [Remote - Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) (optional, for VS Code integration)

## Getting Started

### Using Make Commands

The project includes several Make targets for working with the development container:

```bash
# Start an interactive rholang-shell in the container
make container-rholang-shell

# Build the project in the container
make container-build

# Run tests in the container
make container-test

# Check code quality in the container
make container-check

# Fix code quality issues in the container
make container-fix

# Run the rholang-shell binary in the container
make container-run
```

### Using the Run Script Directly

You can also use the run script directly:

```bash
# Start an interactive rholang-shell in the container
./scripts/run-in-container.sh

# Run a specific command in the container
./scripts/run-in-container.sh make build
./scripts/run-in-container.sh cargo test
./scripts/run-in-container.sh bash -c "cd shell && cargo test"
```

### Using VS Code Remote - Containers

1. Install the [Remote - Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) in VS Code
2. Open the project folder in VS Code
3. Click the green button in the bottom-left corner of the VS Code window
4. Select "Reopen in Container" from the menu
5. VS Code will build the container and open the project inside it

### Using JetBrains IDEs (IntelliJ IDEA, CLion, etc.)

1. Install the [Docker plugin](https://plugins.jetbrains.com/plugin/7724-docker) for your JetBrains IDE
2. Open the project in your JetBrains IDE
3. Set up Docker integration:
   - Go to Settings/Preferences > Build, Execution, Deployment > Docker
   - Click the "+" button to add a Docker configuration
   - Select the appropriate connection type (Unix socket for macOS/Linux, TCP for Windows)
   - Click "Apply" and "OK"
4. Configure a Docker Compose run configuration:
   - Go to Run > Edit Configurations
   - Click the "+" button and select "Docker > Docker Compose"
   - Name the configuration (e.g., "Rholang Dev Container")
   - In "Compose files", select the docker-compose.yml file
   - In "Services", enter "dev"
   - Click "OK"
5. Start the container using the run configuration
6. Use the Terminal in your IDE to execute commands in the container

Alternatively, you can use the Make commands or run script as described above, and then connect your JetBrains IDE to the running container.

## Container Features

The development container includes:

- Rust toolchain with rustfmt and clippy
- Cargo tools: cargo-audit, cargo-tarpaulin
- OpenJDK 17 for JetBrains plugin development
- All dependencies required for building and testing the project

## Serving the rholang WebAssembly demo

The WASM-related code lives in a dedicated crate `rholang-wasm`. You can build and serve the browser-based demo using Docker. This uses a multi-stage image to compile the WASM package with wasm-pack and then runs a small built-in HTTP server (Axum) that serves the static site and exposes an API to run code using the real interpreter.

Quick start:

```bash
# From repository root
docker build -f Dockerfile.wasm -t rholang-wasm .
docker run --rm -p 8080:8080 rholang-wasm
# Open the page:
open http://127.0.0.1:8080/www/index.html
```

Helper script (builds image, runs it, and smoke-tests endpoints):

```bash
./scripts/docker_serve_wasm_shell.sh [--port 8080]
```

Notes:
- The demo site is served under /www, with the generated JS/WASM under /pkg
- The API endpoint is POST /api/run with JSON body { "code": "..." }; it returns { ok: bool, output: string, error: string|null }
- Both the static site and the API are served from the same origin (port 8080)

## Troubleshooting

### Container Build Issues

If you encounter issues building the container:

```bash
# Rebuild the container from scratch
docker-compose build --no-cache dev
```

### Permission Issues

If you encounter permission issues with files created in the container:

```bash
# Fix permissions (run outside the container)
sudo chown -R $(id -u):$(id -g) .
```

### Cargo Cache Issues

If you encounter issues with the cargo cache:

```bash
# Clear the cargo cache
docker volume rm rholang_cargo-cache
```


### Toolchain Notes

- The WebAssembly build image (Dockerfile.wasm) uses the latest Rust nightly toolchain (`rust:nightly`) to ensure up-to-date wasm support in the ecosystem.
- The wasm tooling (wasm-pack and wasm-bindgen-cli) installations are pinned with `--locked` to maintain compatible dependency resolution.

## Running rholang-shell as a TCP service (port 8666)

You can expose the interactive rholang-shell over TCP using the provided helper script. Each incoming connection gets an isolated shell session.

Quick start:

```bash
# From repository root
./scripts/run_shell_service.sh                 # listens on 0.0.0.0:8666
# or choose a different port
./scripts/run_shell_service.sh --port 7777
# or via env
PORT=9000 ./scripts/run_shell_service.sh
```

Connect from another terminal/machine:

```bash
nc 127.0.0.1 8666
# or
telnet 127.0.0.1 8666
```

Notes:
- This script uses socat to bridge TCP <-> a pseudo‑TTY so line editing works.
- Ensure `socat` is installed (macOS: `brew install socat`, Debian/Ubuntu: `sudo apt-get install -y socat`).
- Each connection spawns a fresh `rhosh` process and ends when the client disconnects.
- If you need to expose it in Docker, you can create a simple service that runs this script and publishes port 8666.
