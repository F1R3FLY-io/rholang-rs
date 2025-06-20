#!/bin/bash
set -e

echo "Checking rholang-tree-sitter-proc-macro examples..."

# Change to the proc-macro directory
cd rholang-tree-sitter-proc-macro

# Run examples
echo "Running parse_rholang example..."
cargo run --example parse_rholang --features proc_macros

echo "Running advanced_usage example..."
cargo run --example advanced_usage --features proc_macros

echo "All examples completed successfully!"
