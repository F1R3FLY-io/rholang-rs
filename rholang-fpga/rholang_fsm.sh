#!/bin/bash

# Rholang FSM Core Launcher Script for MiSTer
# This script loads the Rholang FSM core on the MiSTer platform

# Navigate to script directory
cd "$(dirname "$0")"

# Set core name
CORE_NAME="rholang_fsm"

# Create directory for Rholang programs if it doesn't exist
mkdir -p /media/fat/rholang

# Function to display help
function display_help {
    echo "Rholang FSM Core for MiSTer"
    echo "Usage: rholang_fsm.sh [options]"
    echo ""
    echo "Options:"
    echo "  --help    Display this help message"
    echo "  --debug   Enable debug mode"
    echo "  --load=FILE  Load Rholang program from FILE"
    echo ""
    echo "Examples:"
    echo "  rholang_fsm.sh --load=/media/fat/rholang/hello.rho"
    echo ""
}

# Parse command line arguments
DEBUG=0
LOAD_FILE=""

for arg in "$@"; do
    case "$arg" in
        --help)
            display_help
            exit 0
            ;;
        --debug)
            DEBUG=1
            ;;
        --load=*)
            LOAD_FILE="${arg#*=}"
            ;;
        *)
            echo "Unknown option: $arg"
            display_help
            exit 1
            ;;
    esac
done

# Check if core exists
if [ ! -f "${CORE_NAME}.rbf" ]; then
    echo "Error: ${CORE_NAME}.rbf not found."
    exit 1
fi

# Set debug flag if needed
if [ "$DEBUG" -eq 1 ]; then
    echo "Debug mode enabled"
    echo "set debug 1" > /tmp/RHOLANG_DEBUG
else
    echo "set debug 0" > /tmp/RHOLANG_DEBUG
fi

# Set load file if specified
if [ ! -z "$LOAD_FILE" ]; then
    if [ ! -f "$LOAD_FILE" ]; then
        echo "Error: File not found: $LOAD_FILE"
        exit 1
    fi
    echo "Loading Rholang program: $LOAD_FILE"
    echo "set load_file $LOAD_FILE" > /tmp/RHOLANG_LOAD
fi

# Load the core
echo "Loading Rholang FSM Core..."
killall -q MiSTer
/media/fat/MiSTer "${CORE_NAME}.rbf"