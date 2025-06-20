#!/bin/bash

# Common functions and variables for Rholang scripts
# This script is meant to be sourced by other scripts

# Detect if the calling script is being sourced
detect_sourced() {
    # This function should be called by the script that sources this file
    (return 0 2>/dev/null) && sourced=1 || sourced=0

    if [ $sourced -eq 1 ]; then
        # Script is being sourced, warn the user and don't use set -e
        echo "WARNING: This script is designed to be executed directly, not sourced."
        echo "Sourcing this script may cause unexpected behavior."
        echo "Please run it with './$0' instead."
        echo "Continuing without 'set -e' to prevent logging you out..."
    else
        # Script is being executed directly, use set -e
        set -e
    fi
}

# Check if we're in the project root (where Cargo.toml exists)
check_project_root() {
    if [ ! -f "Cargo.toml" ]; then
        echo "Error: This script must be run from the project root directory."
        exit 1
    fi
}

# Create a temporary file to track failures
create_failure_tracker() {
    FAILURES=$(mktemp)
    FAILURES_ABS_PATH=$(realpath "$FAILURES")
    echo "0" > "$FAILURES_ABS_PATH"
    echo "$FAILURES_ABS_PATH"
}

# Function to run a command and report its status
run_command() {
    local cmd="$1"
    local description="$2"
    local failures_path="$3"

    echo "Running $description..."
    if eval "$cmd"; then
        echo "✅ $description passed"
        return 0
    else
        echo "❌ $description failed"
        echo "1" > "$failures_path"
        return 1
    fi
}

# Get all crates in the workspace
get_workspace_crates() {
    # Check if jq is installed
    if ! command -v jq &> /dev/null; then
        echo "⚠️ jq is not installed. Using alternative method to get crates."
        # Alternative method to get crates from Cargo.toml
        grep -A 100 "members" Cargo.toml | grep -v "members" | grep -v "\[" | grep -v "^$" | sed 's/^[ \t]*"\(.*\)",*/\1/' | sed 's/^[ \t]*"\(.*\)"$/\1/'
    else
        # Get all crates in the workspace using jq
        cargo metadata --format-version=1 | jq -r '.workspace_members[] | split(" ")[0]'
    fi
}

# Function to get the crate name from the full path
get_crate_name() {
    local full_path="$1"
    # Extract just the crate name from the full path
    if [[ "$full_path" == path+file* ]]; then
        # For paths like "path+file:///Users/beret/f1r3fly/rholang/shell#0.1.0"
        echo "$full_path" | sed 's|.*/\([^/]*\)#.*|\1|'
    else
        # For simple paths like "shell"
        echo "$full_path"
    fi
}

# Get features required for an example
get_example_features() {
    local crate_name="$1"
    local example="$2"
    
    grep -l "required-features" "$crate_name/Cargo.toml" 2>/dev/null | xargs grep -A 10 "name = \"$example\"" 2>/dev/null | grep "required-features" | sed 's/.*required-features.*\[\(.*\)\].*/\1/' | tr -d ' "' | tr '\n' ' ' | xargs
}

# Check if any failures occurred and clean up
check_failures() {
    local failures_path="$1"
    local success_message="$2"
    local failure_message="$3"
    
    if [ "$(cat "$failures_path")" == "1" ]; then
        echo "❌ $failure_message"
        rm "$failures_path"
        return 1
    else
        echo "✅ $success_message"
        rm "$failures_path"
        return 0
    fi
}