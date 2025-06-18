#!/bin/bash
set -e

echo "Running simplified code quality checks..."

# Create a temporary file to track failures
FAILURES=$(mktemp)
FAILURES_ABS_PATH=$(realpath "$FAILURES")
echo "0" > "$FAILURES_ABS_PATH"
echo "Initial FAILURES value: $(cat "$FAILURES_ABS_PATH")"

# Function to run a command and report its status
run_check() {
    local cmd="$1"
    local description="$2"

    echo "Running $description..."
    if eval "$cmd"; then
        echo "✅ $description passed"
        return 0
    else
        echo "❌ $description failed"
        return 1
    fi
}

# Run rustfmt to check formatting
run_check "cargo fmt --all -- --check" "code formatting check" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: rustfmt failed"; }
echo "FAILURES value after rustfmt: $(cat "$FAILURES_ABS_PATH")"

# Run clippy for linting
run_check "cargo clippy --all-targets --all-features -- -D warnings" "code linting" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: clippy failed"; }
echo "FAILURES value after clippy: $(cat "$FAILURES_ABS_PATH")"

# Run cargo check to ensure code compiles
run_check "cargo check --all" "compilation check" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: cargo check failed"; }
echo "FAILURES value after cargo check: $(cat "$FAILURES_ABS_PATH")"

# Check rholang-tree-sitter-proc-macro examples
if [ -d "rholang-tree-sitter-proc-macro" ]; then
    echo "Checking rholang-tree-sitter-proc-macro examples..."
    echo "FAILURES value before proc-macro checks: $(cat "$FAILURES_ABS_PATH")"

    # Change to the proc-macro directory
    cd rholang-tree-sitter-proc-macro

    # Run examples
    echo "Running parse_rholang example..."
    if cargo run --example parse_rholang --features proc_macros; then
        echo "✅ parse_rholang example passed"
    else
        echo "❌ parse_rholang example failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: parse_rholang example failed"
    fi
    echo "FAILURES value after parse_rholang example: $(cat "$FAILURES_ABS_PATH")"

    echo "Running advanced_usage example..."
    if cargo run --example advanced_usage --features proc_macros; then
        echo "✅ advanced_usage example passed"
    else
        echo "❌ advanced_usage example failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: advanced_usage example failed"
    fi
    echo "FAILURES value after advanced_usage example: $(cat "$FAILURES_ABS_PATH")"

    # Change back to the project root
    cd ..
    echo "FAILURES value after proc-macro checks: $(cat "$FAILURES_ABS_PATH")"
fi

# Check if any failures occurred
echo "Final FAILURES value: $(cat "$FAILURES_ABS_PATH")"
if [ "$(cat "$FAILURES_ABS_PATH")" == "1" ]; then
    echo "❌ Code quality checks failed"
    rm "$FAILURES_ABS_PATH"
    exit 1
else
    echo "✅ All code quality checks passed"
    rm "$FAILURES_ABS_PATH"
    exit 0
fi
