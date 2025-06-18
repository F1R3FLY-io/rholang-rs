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
# Start an interactive shell in the container
make container-shell

# Build the project in the container
make container-build

# Run tests in the container
make container-test

# Check code quality in the container
make container-check

# Fix code quality issues in the container
make container-fix

# Run the shell binary in the container
make container-run
```

### Using the Run Script Directly

You can also use the run script directly:

```bash
# Start an interactive shell in the container
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
