#!/bin/bash

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

# Source common functions
source "$(dirname "$0")/common.sh"

# Detect if this script is being sourced
detect_sourced

# Check if we're in the project root
check_project_root

echo "Running examples for all crates in the workspace..."

# Create a temporary file to track failures
FAILURES_ABS_PATH=$(create_failure_tracker)

# Get all crates in the workspace
CRATES=$(get_workspace_crates)

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
            FEATURES=$(get_example_features "$crate_name" "$example")

            if [ -n "$FEATURES" ]; then
                run_command "cargo run -p $crate_path --example $example --features=\"$FEATURES\"" "example $example for $crate_name (with features: $FEATURES)" "$FAILURES_ABS_PATH"
            else
                run_command "cargo run -p $crate_path --example $example" "example $example for $crate_name" "$FAILURES_ABS_PATH"
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
        rm "$FAILURES_ABS_PATH"
        exit 0
    else
        check_failures "$FAILURES_ABS_PATH" "All examples ran successfully" "Some examples failed"
        exit $?
    fi
fi
