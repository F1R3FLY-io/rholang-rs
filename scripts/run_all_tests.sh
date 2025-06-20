#!/bin/bash

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

# Source common functions
source "$(dirname "$0")/common.sh"

# Detect if this script is being sourced
detect_sourced

# Check if we're in the project root
check_project_root

echo "Running tests for all crates in the workspace..."

# Create a temporary file to track failures
FAILURES_ABS_PATH=$(create_failure_tracker)

# Get all crates in the workspace
CRATES=$(get_workspace_crates)

# Run tests for each crate
for crate_path in $CRATES; do
    # Get the crate name for display and directory operations
    crate_name=$(get_crate_name "$crate_path")

    echo "===== Testing crate: $crate_name ====="

    # Run unit tests
    run_command "cargo test -p $crate_path" "unit tests for $crate_name" "$FAILURES_ABS_PATH"

    # Run doctests
    run_command "cargo test -p $crate_path --doc" "doctests for $crate_name" "$FAILURES_ABS_PATH"

    # Check if the crate has examples
    if [ -d "$crate_name/examples" ]; then
        echo "Found examples directory for $crate_name"

        # Get list of examples
        EXAMPLES=$(find "$crate_name/examples" -name "*.rs" -exec basename {} .rs \;)

        # Run each example
        for example in $EXAMPLES; do
            # Check if the example requires features
            FEATURES=$(get_example_features "$crate_name" "$example")

            if [ -n "$FEATURES" ]; then
                run_command "cargo run -p $crate_path --example $example --features=\"$FEATURES\"" "example $example for $crate_name (with features: $FEATURES)" "$FAILURES_ABS_PATH"
            else
                run_command "cargo run -p $crate_path --example $example" "example $example for $crate_name" "$FAILURES_ABS_PATH"
            fi
        done
    else
        echo "No examples found for $crate_name"
    fi

    echo "===== Completed testing for $crate_name ====="
    echo ""
done

# Check if any failures occurred
check_failures "$FAILURES_ABS_PATH" "All tests passed" "Some tests failed"
exit $?