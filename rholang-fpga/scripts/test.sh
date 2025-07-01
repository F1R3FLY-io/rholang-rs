#!/bin/bash
set -e

# Rholang FSM Core Test Script
# This script runs tests for the Rholang FSM Core

# Display help message
function show_help {
    echo "Rholang FSM Core Test Script"
    echo "Usage: $0 [options]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Display this help message"
    echo "  -v, --verbose  Enable verbose output"
    echo "  -s, --sim      Run simulation tests only"
    echo "  -f, --func     Run functional tests only"
    echo ""
}

# Parse command line arguments
VERBOSE=0
SIM_ONLY=0
FUNC_ONLY=0

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
        -s|--sim)
            SIM_ONLY=1
            shift
            ;;
        -f|--func)
            FUNC_ONLY=1
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
TEST_DIR="tests"
SIM_DIR="$TEST_DIR/sim"
FUNC_DIR="$TEST_DIR/functional"
RESULTS_DIR="test_results"

echo "=== Rholang FSM Core Tests ==="

# Create results directory if it doesn't exist
mkdir -p $RESULTS_DIR

# Check if Quartus is available
if ! command -v quartus_sh &> /dev/null; then
    echo "Error: Quartus Prime is not installed or not in PATH"
    echo "Make sure you're running this script in the Docker container"
    exit 1
fi

# Run simulation tests
if [ "$FUNC_ONLY" -eq 0 ]; then
    echo "Running simulation tests..."
    
    # Create simulation directory if it doesn't exist
    mkdir -p $SIM_DIR
    
    # Check if ModelSim is available
    if ! command -v vsim &> /dev/null; then
        echo "Warning: ModelSim is not installed or not in PATH"
        echo "Skipping simulation tests"
    else
        # Run simulation tests for each module
        for TEST_BENCH in $SIM_DIR/*_tb.v; do
            if [ -f "$TEST_BENCH" ]; then
                TEST_NAME=$(basename "$TEST_BENCH" _tb.v)
                echo "  Testing $TEST_NAME..."
                
                # Compile test bench
                if [ "$VERBOSE" -eq 1 ]; then
                    vlog -work work $TEST_BENCH
                    vlog -work work rtl/${TEST_NAME}.v
                else
                    vlog -work work $TEST_BENCH > $RESULTS_DIR/${TEST_NAME}_compile.log 2>&1
                    vlog -work work rtl/${TEST_NAME}.v >> $RESULTS_DIR/${TEST_NAME}_compile.log 2>&1
                fi
                
                # Run simulation
                if [ "$VERBOSE" -eq 1 ]; then
                    vsim -c work.${TEST_NAME}_tb -do "run -all; quit"
                else
                    vsim -c work.${TEST_NAME}_tb -do "run -all; quit" > $RESULTS_DIR/${TEST_NAME}_sim.log 2>&1
                fi
                
                # Check if simulation was successful
                if grep -q "Test passed" $RESULTS_DIR/${TEST_NAME}_sim.log; then
                    echo "    ✓ Test passed"
                else
                    echo "    ✗ Test failed"
                    if [ "$VERBOSE" -eq 0 ]; then
                        echo "      Check $RESULTS_DIR/${TEST_NAME}_sim.log for details"
                    fi
                    FAILED=1
                fi
            fi
        done
        
        if [ -z "$(ls -A $SIM_DIR)" ]; then
            echo "  No simulation tests found in $SIM_DIR"
        fi
    fi
fi

# Run functional tests
if [ "$SIM_ONLY" -eq 0 ]; then
    echo "Running functional tests..."
    
    # Create functional test directory if it doesn't exist
    mkdir -p $FUNC_DIR
    
    # Run each functional test
    for TEST_SCRIPT in $FUNC_DIR/*.py; do
        if [ -f "$TEST_SCRIPT" ]; then
            TEST_NAME=$(basename "$TEST_SCRIPT" .py)
            echo "  Running $TEST_NAME..."
            
            if [ "$VERBOSE" -eq 1 ]; then
                python3 $TEST_SCRIPT
            else
                python3 $TEST_SCRIPT > $RESULTS_DIR/${TEST_NAME}.log 2>&1
            fi
            
            # Check if test was successful
            if [ $? -eq 0 ]; then
                echo "    ✓ Test passed"
            else
                echo "    ✗ Test failed"
                if [ "$VERBOSE" -eq 0 ]; then
                    echo "      Check $RESULTS_DIR/${TEST_NAME}.log for details"
                fi
                FAILED=1
            fi
        fi
    done
    
    if [ -z "$(ls -A $FUNC_DIR)" ]; then
        echo "  No functional tests found in $FUNC_DIR"
    fi
fi

# Check if any tests failed
if [ "$FAILED" -eq 1 ]; then
    echo "=== Tests Failed ==="
    exit 1
else
    echo "=== All Tests Passed ==="
fi