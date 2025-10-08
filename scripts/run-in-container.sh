#!/bin/bash
set -e

# Script to run commands in the development container
# Usage: ./scripts/run-in-container.sh [command]
# If no command is provided, an interactive shell is started

# Check if Docker is installed
if ! command -v docker &> /dev/null; then
    echo "Error: Docker is not installed or not in PATH"
    exit 1
fi

# Check if Docker Compose is installed
if ! command -v docker-compose &> /dev/null; then
    echo "Error: Docker Compose is not installed or not in PATH"
    exit 1
fi

# Build the container if it doesn't exist
if ! docker-compose ps -q dev &> /dev/null || [ -z "$(docker-compose ps -q dev)" ]; then
    echo "Building development container..."
    docker-compose build dev
fi

# If no command is provided, start an interactive shell
if [ $# -eq 0 ]; then
    echo "Starting interactive shell in development container..."
    docker-compose run --rm dev
else
    # Run the provided command in the container
    echo "Running command in development container: $@"
    docker-compose run --rm dev "$@"
fi