#!/bin/bash
set -e

# Script to check code quality for the Rholang project
# This script runs various tools to ensure code quality:
# - rustfmt: for code formatting
# - clippy: for linting
# - cargo check: for compilation errors
# - cargo audit: for security vulnerabilities (if installed)
# - cargo-tarpaulin: for test coverage

echo "Running code quality checks..."

# Check if we're in the project root (where Cargo.toml exists)
if [ ! -f "Cargo.toml" ]; then
    echo "Error: This script must be run from the project root directory."
    exit 1
fi

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

# Create a temporary file to track failures
FAILURES=$(mktemp)
echo "0" > "$FAILURES"

# Run rustfmt to check formatting
run_check "cargo fmt --all -- --check" "code formatting check" || echo "1" > "$FAILURES"

# Run clippy for linting
run_check "cargo clippy --all-targets --all-features -- -D warnings" "code linting" || echo "1" > "$FAILURES"

# Run cargo check to ensure code compiles
run_check "cargo check --all" "compilation check" || echo "1" > "$FAILURES"

# Run cargo audit if available
if command -v cargo-audit &> /dev/null; then
    run_check "cargo audit" "security audit" || echo "1" > "$FAILURES"
else
    echo "ℹ️ cargo-audit not found, skipping security audit"
    echo "   Install with: cargo install cargo-audit"
fi

# Run cargo-tarpaulin for test coverage if available
if command -v cargo-tarpaulin &> /dev/null; then
    # Run test coverage without requiring 100%
    echo "Running test coverage check..."
    coverage_output=$(cargo tarpaulin --out Stdout)
    coverage_percentage=$(echo "$coverage_output" | grep -o '[0-9]\+\.[0-9]\+%' | head -1)
    echo "✅ Test coverage check completed: $coverage_percentage"
else
    echo "ℹ️ cargo-tarpaulin not found, skipping test coverage check"
    echo "   Install with: cargo install cargo-tarpaulin"
fi

# Check if any failures occurred
if [ "$(cat "$FAILURES")" == "1" ]; then
    echo "❌ Code quality checks failed"
    rm "$FAILURES"
    exit 1
else
    echo "✅ All code quality checks passed"
    rm "$FAILURES"
    exit 0
fi
