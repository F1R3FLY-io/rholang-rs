#!/bin/bash
set -e

# Script to check code quality for the Rholang project
# This script runs various tools to ensure code quality:
# - rustfmt: for code formatting
# - clippy: for linting
# - cargo check: for compilation errors
# - cargo audit: for security vulnerabilities (if installed)
# - cargo-tarpaulin: for test coverage
# - checkstyle: for Java code style checking
# - pmd: for Java code analysis
# - jacoco: for Java test coverage

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
FAILURES_ABS_PATH=$(realpath "$FAILURES")
echo "0" > "$FAILURES_ABS_PATH"
echo "Initial FAILURES value: $(cat "$FAILURES_ABS_PATH")"

# Run rustfmt to check formatting
run_check "cargo fmt --all -- --check" "code formatting check" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: rustfmt failed"; }
echo "FAILURES value after rustfmt: $(cat "$FAILURES_ABS_PATH")"

# Run clippy for linting
run_check "cargo clippy --all-targets --all-features -- -D warnings" "code linting" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: clippy failed"; }
echo "FAILURES value after clippy: $(cat "$FAILURES_ABS_PATH")"

# Run cargo check to ensure code compiles
run_check "cargo check --all" "compilation check" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: cargo check failed"; }
echo "FAILURES value after cargo check: $(cat "$FAILURES_ABS_PATH")"

# Run cargo audit if available
if command -v cargo-audit &> /dev/null; then
    run_check "cargo audit" "security audit" || { echo "1" > "$FAILURES_ABS_PATH"; echo "DEBUG: cargo audit failed"; }
    echo "FAILURES value after cargo audit: $(cat "$FAILURES_ABS_PATH")"
else
    echo "ℹ️ cargo-audit not found, skipping security audit"
    echo "   Install with: cargo install cargo-audit"
fi

# Run cargo-tarpaulin for test coverage if available
if command -v cargo-tarpaulin &> /dev/null; then
    # Run test coverage without requiring 100%
    echo "Running test coverage check..."
    coverage_output=$(cargo tarpaulin --out Stdout)
    coverage_percentage=$(echo "$coverage_output" | grep -o '[0-9]\+\.[0-9]\+% coverage' | head -1 | sed 's/ coverage//')
    if [ -z "$coverage_percentage" ]; then
        # Try alternative pattern if the first one doesn't match
        coverage_percentage=$(echo "$coverage_output" | grep -o '[0-9]\+\.[0-9]\+%' | head -1)
    fi
    if [ -z "$coverage_percentage" ]; then
        coverage_percentage="0.00%"
    fi
    echo "✅ Test coverage check completed: $coverage_percentage"
    echo "FAILURES value after test coverage: $(cat "$FAILURES_ABS_PATH")"
else
    echo "ℹ️ cargo-tarpaulin not found, skipping test coverage check"
    echo "   Install with: cargo install cargo-tarpaulin"
fi

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
        echo "FAILURES value after parse_rholang: $(cat "$FAILURES_ABS_PATH")"
    fi
    echo "FAILURES value after parse_rholang example: $(cat "$FAILURES_ABS_PATH")"

    echo "Running advanced_usage example..."
    if cargo run --example advanced_usage --features proc_macros; then
        echo "✅ advanced_usage example passed"
    else
        echo "❌ advanced_usage example failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: advanced_usage example failed"
        echo "FAILURES value after advanced_usage: $(cat "$FAILURES_ABS_PATH")"
    fi
    echo "FAILURES value after advanced_usage example: $(cat "$FAILURES_ABS_PATH")"

    # Change back to the project root
    cd ..
    echo "FAILURES value after proc-macro checks: $(cat "$FAILURES_ABS_PATH")"
else
    echo "ℹ️ rholang-tree-sitter-proc-macro directory not found, skipping proc-macro checks"
fi

# Check JetBrains plugin code quality
if [ -d "rholang-jetbrains-plugin" ]; then
    echo "Checking JetBrains plugin code quality..."

    # Change to the plugin directory
    cd rholang-jetbrains-plugin

    # Run Checkstyle
    echo "Running Checkstyle..."
    if ./gradlew checkstyleMain checkstyleTest; then
        echo "✅ Checkstyle passed"
    else
        echo "❌ Checkstyle failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: Checkstyle failed"
    fi

    # Run PMD
    echo "Running PMD..."
    if ./gradlew pmdMain pmdTest; then
        echo "✅ PMD passed"
    else
        echo "❌ PMD failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: PMD failed"
    fi

    # Run tests with JaCoCo coverage
    echo "Running tests with JaCoCo coverage..."
    if ./gradlew test jacocoTestReport; then
        echo "✅ Tests passed"
        echo "JaCoCo coverage report is available at: rholang-jetbrains-plugin/build/reports/jacoco/test/html/index.html"
    else
        echo "❌ Tests failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: JaCoCo tests failed"
    fi

    # Build the plugin
    echo "Building the plugin..."
    if ./gradlew buildPlugin; then
        echo "✅ Plugin build successful"
    else
        echo "❌ Plugin build failed"
        echo "1" > "$FAILURES_ABS_PATH"
        echo "DEBUG: Plugin build failed"
    fi

    # Change back to the project root
    cd ..
else
    echo "ℹ️ JetBrains plugin directory not found, skipping plugin checks"
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
