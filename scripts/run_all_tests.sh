#!/bin/bash
set -e

# Script to run tests for all crates in the Rholang workspace
# This script runs:
# - Unit tests
# - Integration tests
# - Doctests
# - Examples (if available)
# for each crate in the workspace
#
# Usage:
#   ./scripts/run_all_tests.sh
#
# Requirements:
#   - jq (optional, will use alternative method if not available)
#
# The script will:
#   1. Identify all crates in the workspace
#   2. For each crate, run unit tests and doctests
#   3. For crates with examples, run each example
#   4. Report the overall status at the end
#
# Exit codes:
#   0 - All tests passed
#   1 - Some tests failed

echo "Running tests for all crates in the workspace..."

# Check if we're in the project root (where Cargo.toml exists)
if [ ! -f "Cargo.toml" ]; then
    echo "Error: This script must be run from the project root directory."
    exit 1
fi

# Create a temporary file to track failures
FAILURES=$(mktemp)
FAILURES_ABS_PATH=$(realpath "$FAILURES")
echo "0" > "$FAILURES_ABS_PATH"

# Function to run a command and report its status
run_test() {
    local cmd="$1"
    local description="$2"

    echo "Running $description..."
    if eval "$cmd"; then
        echo "✅ $description passed"
        return 0
    else
        echo "❌ $description failed"
        echo "1" > "$FAILURES_ABS_PATH"
        return 1
    fi
}

# Check if jq is installed
if ! command -v jq &> /dev/null; then
    echo "⚠️ jq is not installed. Using alternative method to get crates."
    # Alternative method to get crates from Cargo.toml
    CRATES=$(grep -A 100 "members" Cargo.toml | grep -v "members" | grep -v "\[" | grep -v "^$" | sed 's/^[ \t]*"\(.*\)",*/\1/' | sed 's/^[ \t]*"\(.*\)"$/\1/')
else
    # Get all crates in the workspace using jq
    CRATES=$(cargo metadata --format-version=1 | jq -r '.workspace_members[] | split(" ")[0]')
fi

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

# Run tests for each crate
for crate_path in $CRATES; do
    # Get the crate name for display and directory operations
    crate_name=$(get_crate_name "$crate_path")

    echo "===== Testing crate: $crate_name ====="

    # Run unit tests
    run_test "cargo test -p $crate_path" "unit tests for $crate_name"

    # Run doctests
    run_test "cargo test -p $crate_path --doc" "doctests for $crate_name"

    # Check if the crate has examples
    if [ -d "$crate_name/examples" ]; then
        echo "Found examples directory for $crate_name"

        # Get list of examples
        EXAMPLES=$(find "$crate_name/examples" -name "*.rs" -exec basename {} .rs \;)

        # Run each example
        for example in $EXAMPLES; do
            # Check if the example requires features
            FEATURES=$(grep -l "required-features" "$crate_name/Cargo.toml" | xargs grep -A 10 "name = \"$example\"" | grep "required-features" | sed 's/.*required-features.*\[\(.*\)\].*/\1/' | tr -d ' "')

            if [ -n "$FEATURES" ]; then
                run_test "cargo run -p $crate_path --example $example --features $FEATURES" "example $example for $crate_name (with features: $FEATURES)"
            else
                run_test "cargo run -p $crate_path --example $example" "example $example for $crate_name"
            fi
        done
    else
        echo "No examples found for $crate_name"
    fi

    echo "===== Completed testing for $crate_name ====="
    echo ""
done

# Check if any failures occurred
if [ "$(cat "$FAILURES_ABS_PATH")" == "1" ]; then
    echo "❌ Some tests failed"
    rm "$FAILURES_ABS_PATH"
    exit 1
else
    echo "✅ All tests passed"
    rm "$FAILURES_ABS_PATH"
    exit 0
fi
