#!/bin/bash
set -e

# Script to run all examples from all crates in the Rholang workspace
#
# Usage:
#   ./scripts/run_all_examples.sh
#
# Requirements:
#   - jq (optional, will use alternative method if not available)
#
# The script will:
#   1. Identify all crates in the workspace
#   2. For each crate, check if it has examples
#   3. Run all examples for each crate that has them
#   4. Report the overall status at the end
#
# Exit codes:
#   0 - All examples ran successfully
#   1 - Some examples failed

echo "Running examples for all crates in the workspace..."

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
run_example() {
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

# Track the total number of examples found and run
TOTAL_EXAMPLES=0
RUN_EXAMPLES=0

# Run examples for each crate
for crate_path in $CRATES; do
    # Get the crate name for display and directory operations
    crate_name=$(get_crate_name "$crate_path")

    echo "===== Checking examples for crate: $crate_name ====="

    # Check if the crate has examples
    if [ -d "$crate_name/examples" ]; then
        echo "Found examples directory for $crate_name"

        # Get list of examples
        EXAMPLES=$(find "$crate_name/examples" -name "*.rs" -exec basename {} .rs \;)
        
        # Count examples
        example_count=$(echo "$EXAMPLES" | wc -w)
        TOTAL_EXAMPLES=$((TOTAL_EXAMPLES + example_count))
        
        if [ -z "$EXAMPLES" ]; then
            echo "No example files found in $crate_name/examples"
            continue
        fi

        echo "Found $example_count example(s) in $crate_name"

        # Run each example
        for example in $EXAMPLES; do
            # Check if the example requires features
            FEATURES=$(grep -l "required-features" "$crate_name/Cargo.toml" 2>/dev/null | xargs grep -A 10 "name = \"$example\"" 2>/dev/null | grep "required-features" | sed 's/.*required-features.*\[\(.*\)\].*/\1/' | tr -d ' "')

            if [ -n "$FEATURES" ]; then
                run_example "cargo run -p $crate_path --example $example --features $FEATURES" "example $example for $crate_name (with features: $FEATURES)"
            else
                run_example "cargo run -p $crate_path --example $example" "example $example for $crate_name"
            fi
            
            # Increment the count of run examples
            RUN_EXAMPLES=$((RUN_EXAMPLES + 1))
        done
    else
        echo "No examples directory found for $crate_name"
    fi

    echo "===== Completed checking examples for $crate_name ====="
    echo ""
done

echo "Summary: Found $TOTAL_EXAMPLES examples across all crates, ran $RUN_EXAMPLES examples"

# Check if any failures occurred
if [ "$(cat "$FAILURES_ABS_PATH")" == "1" ]; then
    echo "❌ Some examples failed"
    rm "$FAILURES_ABS_PATH"
    exit 1
else
    if [ "$RUN_EXAMPLES" -eq 0 ]; then
        echo "⚠️ No examples were found or run"
    else
        echo "✅ All examples ran successfully"
    fi
    rm "$FAILURES_ABS_PATH"
    exit 0
fi