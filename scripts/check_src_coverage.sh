#!/bin/bash

# Script to check only source code coverage (excluding tests)
# Usage: ./scripts/check_src_coverage.sh [output_format] [output_dir]

# Default values
OUTPUT_FORMAT=${1:-"Stdout"}
OUTPUT_DIR=${2:-"coverage"}

# Run tarpaulin with focus on src files only
if [ "$OUTPUT_FORMAT" = "Stdout" ]; then
  cargo tarpaulin --include-files "*/src/*" --out "$OUTPUT_FORMAT"
else
  cargo tarpaulin --include-files "*/src/*" --out "$OUTPUT_FORMAT" --output-dir "$OUTPUT_DIR"
  echo "Coverage report generated in $OUTPUT_DIR directory"
fi

# Print a reminder about the command used
echo ""
echo "Command used: cargo tarpaulin --include-files \"*/src/*\" --out $OUTPUT_FORMAT"
if [ "$OUTPUT_FORMAT" != "Stdout" ]; then
  echo "              --output-dir $OUTPUT_DIR"
fi