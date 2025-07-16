#!/bin/bash
set -e

# Rholang FSM Core Debug Script
# This script provides debugging tools for the Rholang FSM Core

# Display help message
function show_help {
    echo "Rholang FSM Core Debug Script"
    echo "Usage: $0 [options] [module]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Display this help message"
    echo "  -s, --signal   Open SignalTap Logic Analyzer"
    echo "  -w, --wave     Open waveform viewer"
    echo "  -r, --rtl      Open RTL viewer"
    echo "  -t, --timing   Open Timing Analyzer"
    echo ""
    echo "Examples:"
    echo "  $0 -s          Open SignalTap Logic Analyzer"
    echo "  $0 -w fpu      Open waveform viewer for FPU module"
    echo "  $0 -r ccn      Open RTL viewer for CCN module"
    echo ""
}

# Parse command line arguments
SIGNALTAP=0
WAVEFORM=0
RTL=0
TIMING=0
MODULE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        -h|--help)
            show_help
            exit 0
            ;;
        -s|--signal)
            SIGNALTAP=1
            shift
            ;;
        -w|--wave)
            WAVEFORM=1
            shift
            ;;
        -r|--rtl)
            RTL=1
            shift
            ;;
        -t|--timing)
            TIMING=1
            shift
            ;;
        *)
            if [ -z "$MODULE" ]; then
                MODULE="$1"
                shift
            else
                echo "Unknown option: $1"
                show_help
                exit 1
            fi
            ;;
    esac
done

# Set up environment
QUARTUS_PROJECT="rholang_fsm"
OUTPUT_DIR="output_files"
SIM_DIR="simulation"
DEBUG_DIR="debug"

echo "=== Rholang FSM Core Debug ==="

# Create debug directory if it doesn't exist
mkdir -p $DEBUG_DIR

# Check if Quartus is available
if ! command -v quartus_sh &> /dev/null; then
    echo "Error: Quartus Prime is not installed or not in PATH"
    echo "Make sure you're running this script in the Docker container"
    exit 1
fi

# Check if X11 forwarding is working
if [ -z "$DISPLAY" ]; then
    echo "Error: X11 forwarding is not set up"
    echo "Make sure you're running the Docker container with X11 forwarding enabled"
    echo "Example: docker-compose run --rm -e DISPLAY=\$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix debug"
    exit 1
fi

# Open SignalTap Logic Analyzer
if [ "$SIGNALTAP" -eq 1 ]; then
    echo "Opening SignalTap Logic Analyzer..."
    
    # Check if SignalTap file exists
    if [ -f "${QUARTUS_PROJECT}.stp" ]; then
        quartus_stpw ${QUARTUS_PROJECT}.stp &
    else
        # Create a new SignalTap file
        echo "Creating new SignalTap file..."
        quartus_stp --new ${QUARTUS_PROJECT}.stp &
    fi
fi

# Open waveform viewer
if [ "$WAVEFORM" -eq 1 ]; then
    echo "Opening waveform viewer..."
    
    # Check if ModelSim is available
    if ! command -v vsim &> /dev/null; then
        echo "Error: ModelSim is not installed or not in PATH"
        exit 1
    fi
    
    # Check if module is specified
    if [ -z "$MODULE" ]; then
        echo "Error: Module name is required for waveform viewer"
        echo "Example: $0 -w fpu"
        exit 1
    fi
    
    # Check if waveform file exists
    if [ -f "$SIM_DIR/${MODULE}_tb.wlf" ]; then
        vsim -view $SIM_DIR/${MODULE}_tb.wlf &
    else
        echo "Error: Waveform file not found for module $MODULE"
        echo "Run simulation first: ./scripts/test.sh -s"
        exit 1
    fi
fi

# Open RTL viewer
if [ "$RTL" -eq 1 ]; then
    echo "Opening RTL viewer..."
    
    # Check if module is specified
    if [ -z "$MODULE" ]; then
        # Open RTL viewer for the whole project
        quartus_map $QUARTUS_PROJECT --rtl=on --source=on &
    else
        # Check if module file exists
        if [ -f "rtl/${MODULE}.v" ]; then
            # Open RTL viewer for the specified module
            quartus_map $QUARTUS_PROJECT --rtl=on --source=on --rev=$MODULE &
        else
            echo "Error: Module file not found: rtl/${MODULE}.v"
            exit 1
        fi
    fi
fi

# Open Timing Analyzer
if [ "$TIMING" -eq 1 ]; then
    echo "Opening Timing Analyzer..."
    
    # Check if timing netlist exists
    if [ -f "$OUTPUT_DIR/${QUARTUS_PROJECT}.sta.rpt" ]; then
        quartus_sta $QUARTUS_PROJECT &
    else
        echo "Error: Timing netlist not found"
        echo "Run compilation first: ./scripts/build.sh"
        exit 1
    fi
fi

# If no debug option is specified, show help
if [ "$SIGNALTAP" -eq 0 ] && [ "$WAVEFORM" -eq 0 ] && [ "$RTL" -eq 0 ] && [ "$TIMING" -eq 0 ]; then
    show_help
    exit 0
fi

echo "Debug tools launched. Close the windows when finished."