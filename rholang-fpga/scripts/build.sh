#!/bin/bash
set -e

# Rholang FSM Core Build Script
# This script compiles the Rholang FSM Core for MiSTer FPGA

# Display help message
function show_help {
    echo "Rholang FSM Core Build Script"
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Display this help message"
    echo "  -c, --clean    Clean build directory before compilation"
    echo "  -v, --verbose  Enable verbose output"
    echo ""
}

# Parse command line arguments
CLEAN=0
VERBOSE=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        -c|--clean)
            CLEAN=1
            shift
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        *)
            echo "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Set up environment
QUARTUS_PROJECT="rholang_fsm"
OUTPUT_DIR="output_files"

echo "=== Rholang FSM Core Build ==="
echo "Project: $QUARTUS_PROJECT"

# Clean build directory if requested
if [ "$CLEAN" -eq 1 ]; then
    echo "Cleaning build directory..."
    rm -rf $OUTPUT_DIR
    rm -rf db
    rm -rf incremental_db
    rm -f *.rpt
    rm -f *.summary
    rm -f *.smsg
    rm -f *.done
    rm -f *.jdi
    rm -f *.pin
    rm -f *.sof
    rm -f *.rbf
fi

# Create output directory if it doesn't exist
mkdir -p $OUTPUT_DIR

# Check if Quartus is available
if ! command -v quartus_sh &> /dev/null; then
    echo "Error: Quartus Prime is not installed or not in PATH"
    echo "Make sure you're running this script in the Docker container"
    exit 1
fi

# Compile the project
echo "Starting compilation..."
if [ "$VERBOSE" -eq 1 ]; then
    quartus_sh --flow compile $QUARTUS_PROJECT
else
    quartus_sh --flow compile $QUARTUS_PROJECT > $OUTPUT_DIR/build.log 2>&1
fi

# Check if compilation was successful
if [ $? -eq 0 ]; then
    echo "Compilation successful!"
    
    # Generate RBF file for MiSTer
    echo "Generating RBF file..."
    quartus_cpf -c -o bitstream_compression=on $OUTPUT_DIR/$QUARTUS_PROJECT.sof $OUTPUT_DIR/$QUARTUS_PROJECT.rbf
    
    if [ $? -eq 0 ]; then
        echo "RBF file generated successfully: $OUTPUT_DIR/$QUARTUS_PROJECT.rbf"
    else
        echo "Error: Failed to generate RBF file"
        exit 1
    fi
else
    echo "Error: Compilation failed"
    if [ "$VERBOSE" -eq 0 ]; then
        echo "Check $OUTPUT_DIR/build.log for details"
    fi
    exit 1
fi

echo "=== Build Complete ==="
echo "Output files:"
echo "  SOF: $OUTPUT_DIR/$QUARTUS_PROJECT.sof"
echo "  RBF: $OUTPUT_DIR/$QUARTUS_PROJECT.rbf"