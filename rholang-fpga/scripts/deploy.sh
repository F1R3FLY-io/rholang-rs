#!/bin/bash
set -e

# Rholang FSM Core Deploy Script
# This script deploys the Rholang FSM Core to a MiSTer FPGA

# Display help message
function show_help {
    echo "Rholang FSM Core Deploy Script"
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -h, --help           Display this help message"
    echo "  -v, --verbose        Enable verbose output"
    echo "  -b, --build          Build before deploying"
    echo "  -H, --host HOST      MiSTer hostname or IP address (default: mister)"
    echo "  -u, --user USER      SSH username (default: root)"
    echo "  -p, --port PORT      SSH port (default: 22)"
    echo "  -k, --key KEY_FILE   SSH private key file"
    echo "  -s, --samples        Deploy sample Rholang programs"
    echo ""
}

# Parse command line arguments
VERBOSE=0
BUILD=0
HOST="mister"
USER="root"
PORT="22"
KEY_FILE=""
DEPLOY_SAMPLES=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        -v|--verbose)
            VERBOSE=1
            shift
            ;;
        -b|--build)
            BUILD=1
            shift
            ;;
        -H|--host)
            HOST="$2"
            shift 2
            ;;
        -u|--user)
            USER="$2"
            shift 2
            ;;
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        -k|--key)
            KEY_FILE="$2"
            shift 2
            ;;
        -s|--samples)
            DEPLOY_SAMPLES=1
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
SAMPLES_DIR="samples"
SSH_OPTS="-o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null"

if [ ! -z "$KEY_FILE" ]; then
    SSH_OPTS="$SSH_OPTS -i $KEY_FILE"
fi

echo "=== Rholang FSM Core Deploy ==="
echo "Target: $USER@$HOST:$PORT"

# Build the project if requested
if [ "$BUILD" -eq 1 ]; then
    echo "Building project before deployment..."
    ./scripts/build.sh
fi

# Check if RBF file exists
if [ ! -f "$OUTPUT_DIR/$QUARTUS_PROJECT.rbf" ]; then
    echo "Error: RBF file not found: $OUTPUT_DIR/$QUARTUS_PROJECT.rbf"
    echo "Run build script first: ./scripts/build.sh"
    exit 1
fi

# Check if launcher script exists
if [ ! -f "$QUARTUS_PROJECT.sh" ]; then
    echo "Error: Launcher script not found: $QUARTUS_PROJECT.sh"
    exit 1
fi

# Deploy RBF file to MiSTer
echo "Deploying RBF file to MiSTer..."
if [ "$VERBOSE" -eq 1 ]; then
    scp $SSH_OPTS -P $PORT $OUTPUT_DIR/$QUARTUS_PROJECT.rbf $USER@$HOST:/media/fat/
else
    scp $SSH_OPTS -P $PORT $OUTPUT_DIR/$QUARTUS_PROJECT.rbf $USER@$HOST:/media/fat/ > /dev/null 2>&1
fi

# Deploy launcher script to MiSTer
echo "Deploying launcher script to MiSTer..."
if [ "$VERBOSE" -eq 1 ]; then
    scp $SSH_OPTS -P $PORT $QUARTUS_PROJECT.sh $USER@$HOST:/media/fat/
    ssh $SSH_OPTS -p $PORT $USER@$HOST "chmod +x /media/fat/$QUARTUS_PROJECT.sh"
else
    scp $SSH_OPTS -P $PORT $QUARTUS_PROJECT.sh $USER@$HOST:/media/fat/ > /dev/null 2>&1
    ssh $SSH_OPTS -p $PORT $USER@$HOST "chmod +x /media/fat/$QUARTUS_PROJECT.sh" > /dev/null 2>&1
fi

# Create directory for Rholang programs
echo "Creating directory for Rholang programs..."
if [ "$VERBOSE" -eq 1 ]; then
    ssh $SSH_OPTS -p $PORT $USER@$HOST "mkdir -p /media/fat/rholang"
else
    ssh $SSH_OPTS -p $PORT $USER@$HOST "mkdir -p /media/fat/rholang" > /dev/null 2>&1
fi

# Deploy sample Rholang programs if requested
if [ "$DEPLOY_SAMPLES" -eq 1 ]; then
    echo "Deploying sample Rholang programs..."
    if [ "$VERBOSE" -eq 1 ]; then
        scp $SSH_OPTS -P $PORT $SAMPLES_DIR/*.rho $USER@$HOST:/media/fat/rholang/
    else
        scp $SSH_OPTS -P $PORT $SAMPLES_DIR/*.rho $USER@$HOST:/media/fat/rholang/ > /dev/null 2>&1
    fi
fi

echo "=== Deployment Complete ==="
echo "To run the core on MiSTer:"
echo "  1. Connect to MiSTer via SSH: ssh $USER@$HOST -p $PORT"
echo "  2. Run the launcher script: /media/fat/$QUARTUS_PROJECT.sh"
echo "  3. To run a specific Rholang program: /media/fat/$QUARTUS_PROJECT.sh --load=/media/fat/rholang/hello.rho"