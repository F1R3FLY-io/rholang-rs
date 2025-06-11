#!/bin/bash
set -e

# Script to fix code quality issues for the Rholang project
# This script runs various tools to fix code quality issues:
# - rustfmt: for code formatting
# - clippy: for linting (with automatic fixes where possible)

echo "Fixing code quality issues..."

# Check if we're in the project root (where Cargo.toml exists)
if [ ! -f "Cargo.toml" ]; then
    echo "Error: This script must be run from the project root directory."
    exit 1
fi

# Function to run a command and report its status
run_fix() {
    local cmd="$1"
    local description="$2"

    echo "Running $description..."
    if eval "$cmd"; then
        echo "✅ $description completed"
        return 0
    else
        echo "⚠️ $description encountered issues"
        return 1
    fi
}

# Format code with rustfmt
run_fix "cargo fmt --all" "code formatting"

# Run clippy with automatic fixes where possible
run_fix "cargo clippy --all-targets --all-features --fix --allow-dirty -- -D warnings" "code linting fixes"

echo "✅ Code quality fixes applied"
echo "Run './scripts/check_code_quality.sh' to verify all issues are resolved"
