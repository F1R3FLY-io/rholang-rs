#!/bin/bash
set -e

# Check if rustfmt and clippy are installed
if ! command -v rustfmt &> /dev/null; then
    echo "rustfmt is not installed. Installing..."
    rustup component add rustfmt
fi

if ! command -v cargo-clippy &> /dev/null; then
    echo "clippy is not installed. Installing..."
    rustup component add clippy
fi

echo "Formatting code..."
cargo fmt

echo "Fixing clippy lints where possible..."
cargo clippy --fix --allow-dirty --allow-staged

echo "Code quality fixes applied!"