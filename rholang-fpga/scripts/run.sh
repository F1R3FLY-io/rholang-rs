#!/bin/bash
set -e

# Rholang FSM Core Run Script
# This script runs the Rholang FSM Core in simulation mode

# Display help message
function show_help {
    echo "Rholang FSM Core Run Script"
    echo "Usage: $0 [options] [program]"
    echo ""
    echo "Options:"
    echo "  -h, --help     Display this help message"
    echo "  -v, --verbose  Enable verbose output"
    echo "  -d, --debug    Enable debug mode"
    echo ""
    echo "Examples:"
    echo "  $0 samples/hello.rho       Run hello world program"
    echo "  $0 -d samples/fibonacci.rho Run fibonacci program with debug mode"
    echo ""
}

# Parse command line arguments
VERBOSE=0
DEBUG=0
PROGRAM=""

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
        -d|--debug)
            DEBUG=1
            shift
            ;;
        *)
            if [ -z "$PROGRAM" ]; then
                PROGRAM="$1"
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
SIM_DIR="simulation"
OUTPUT_DIR="output_files"

echo "=== Rholang FSM Core Run ==="

# Create simulation directory if it doesn't exist
mkdir -p $SIM_DIR

# Check if ModelSim is available
if ! command -v vsim &> /dev/null; then
    echo "Error: ModelSim is not installed or not in PATH"
    echo "Make sure you're running this script in the Docker container"
    exit 1
fi

# Check if program file is specified
if [ -z "$PROGRAM" ]; then
    echo "Error: No Rholang program specified"
    show_help
    exit 1
fi

# Check if program file exists
if [ ! -f "$PROGRAM" ]; then
    echo "Error: Program file not found: $PROGRAM"
    exit 1
fi

# Check if program file is a Rholang file
if [[ "$PROGRAM" != *.rho ]]; then
    echo "Error: Program file must be a Rholang file (*.rho)"
    exit 1
fi

# Parse Rholang program
echo "Parsing Rholang program: $PROGRAM"
PROGRAM_NAME=$(basename "$PROGRAM" .rho)

# In a real implementation, this would parse the Rholang program into a format
# that can be simulated by the FSM core. For now, we'll just copy the file to
# the simulation directory.
cp "$PROGRAM" "$SIM_DIR/$PROGRAM_NAME.rho"

# Create simulation testbench
echo "Creating simulation testbench..."
cat > "$SIM_DIR/${PROGRAM_NAME}_tb.v" << EOF
\`timescale 1ns/1ps

module ${PROGRAM_NAME}_tb;
    // Clock and reset
    reg clk;
    reg reset;
    
    // Instantiate the Rholang FSM Core
    Rholang_FSM_Core dut (
        .clk(clk),
        .reset(reset),
        // Other ports would be connected here
        .debug_leds()
    );
    
    // Clock generation
    initial begin
        clk = 0;
        forever #5 clk = ~clk; // 100 MHz clock
    end
    
    // Test sequence
    initial begin
        // Initialize
        reset = 1;
        #100;
        reset = 0;
        
        // Load program
        // In a real implementation, this would load the parsed Rholang program
        // into the FSM core's memory.
        #100;
        
        // Run simulation for a while
        #10000;
        
        // End simulation
        \$display("Simulation complete");
        \$finish;
    end
    
    // Debug output
    initial begin
        \$dumpfile("${SIM_DIR}/${PROGRAM_NAME}.vcd");
        \$dumpvars(0, ${PROGRAM_NAME}_tb);
    end
endmodule
EOF

# Run simulation
echo "Running simulation..."
if [ "$VERBOSE" -eq 1 ]; then
    # Compile testbench and core
    vlog -work work $SIM_DIR/${PROGRAM_NAME}_tb.v
    vlog -work work rtl/*.v
    
    # Run simulation
    if [ "$DEBUG" -eq 1 ]; then
        vsim -gui work.${PROGRAM_NAME}_tb -do "run -all"
    else
        vsim -c work.${PROGRAM_NAME}_tb -do "run -all; quit"
    fi
else
    # Compile testbench and core
    vlog -work work $SIM_DIR/${PROGRAM_NAME}_tb.v > $SIM_DIR/${PROGRAM_NAME}_compile.log 2>&1
    vlog -work work rtl/*.v >> $SIM_DIR/${PROGRAM_NAME}_compile.log 2>&1
    
    # Run simulation
    if [ "$DEBUG" -eq 1 ]; then
        vsim -gui work.${PROGRAM_NAME}_tb -do "run -all" > $SIM_DIR/${PROGRAM_NAME}_sim.log 2>&1
    else
        vsim -c work.${PROGRAM_NAME}_tb -do "run -all; quit" > $SIM_DIR/${PROGRAM_NAME}_sim.log 2>&1
    fi
fi

# Check if simulation was successful
if [ $? -eq 0 ]; then
    echo "Simulation successful!"
    
    # In a real implementation, this would display the output of the Rholang program
    echo "Program output:"
    echo "  (In a real implementation, this would show the actual output of the Rholang program)"
    
    # If debug mode is enabled, open waveform viewer
    if [ "$DEBUG" -eq 1 ]; then
        echo "Opening waveform viewer..."
        gtkwave $SIM_DIR/${PROGRAM_NAME}.vcd &
    fi
else
    echo "Error: Simulation failed"
    if [ "$VERBOSE" -eq 0 ]; then
        echo "Check $SIM_DIR/${PROGRAM_NAME}_sim.log for details"
    fi
    exit 1
fi

echo "=== Run Complete ==="