# Implementation Guide: Rholang FSM for MiSTer FPGA

## Introduction

This implementation guide provides step-by-step instructions for creating a Rholang Finite State Machine (FSM) core for the MiSTer FPGA platform. The MiSTer platform is an open-source project that uses the DE10-Nano development board with a Cyclone V FPGA to implement various computing systems. This guide focuses on implementing the Rholang FSM model, which represents the execution semantics of the Rholang concurrent programming language.

## Prerequisites

Before beginning this implementation, ensure you have:

1. **DE10-Nano Board** with MiSTer setup
2. **Development Environment**:
   - Quartus Prime (Lite Edition 17.0 or later)
   - Intel FPGA Software Development Kit
   - MiSTer development tools
3. **Knowledge Requirements**:
   - Verilog HDL programming
   - FPGA design and synthesis
   - Finite State Machine theory
   - Basic understanding of Rholang and concurrent programming

## Implementation Roadmap

This implementation will proceed in several phases:

1. **Development Environment Setup**
2. **Core Architecture Design**
3. **FSM Component Implementation**
4. **Integration with MiSTer**
5. **Testing and Validation**
6. **Optimization**
7. **Documentation and Deployment**

## Phase 1: Development Environment Setup

### Setting Up MiSTer Development Environment

1. **Clone MiSTer Repository**
   ```bash
   git clone https://github.com/MiSTer-devel/Main_MiSTer.git
   cd Main_MiSTer
   ```

2. **Create Core Directory**
   ```bash
   mkdir -p cores/rholang_fsm
   cd cores/rholang_fsm
   ```

3. **Setup Quartus Project**
   - Create a new Quartus project targeting Cyclone V (5CSEBA6U23I7)
   - Configure project settings for MiSTer compatibility
   - Import MiSTer template files

4. **Install Supporting Tools**
   ```bash
   # Install tools for Rholang parsing (if using Python)
   pip install antlr4-python3-runtime

   # Setup C++ build environment for host tools
   sudo apt-get install build-essential cmake
   ```

## Phase 2: Core Architecture Design

### System Architecture Overview

The Rholang FSM core consists of these main components:

1. **FSM Processing Units (FPUs)**
   - Hardware modules implementing the FSM execution model
   - Multiple instances for parallel process execution

2. **Channel Communication Network (CCN)**
   - Implements message passing between processes
   - Manages channel creation and message routing

3. **Process Creation and Management Unit (PCMU)**
   - Handles process creation, forking, and termination
   - Manages process lifecycle and scheduling

4. **Memory Management Unit (MMU)**
   - Manages memory allocation for processes
   - Implements garbage collection for unused resources

5. **Linux Interface Controller (LIC)**
   - Provides interface with MiSTer Linux OS
   - Handles loading programs and reporting status

### Design Files Structure

Create the following directory structure for your project:

```
rholang_fsm/
├── rtl/
│   ├── fsm_processing_unit.v
│   ├── channel_communication.v
│   ├── process_management.v
│   ├── memory_management.v
│   ├── linux_interface.v
│   └── rholang_fsm_top.v
├── sim/
│   ├── testbench.v
│   └── test_programs/
├── tools/
│   ├── rholang_parser.cpp
│   └── fsm_encoder.cpp
├── docs/
│   ├── architecture.md
│   └── user_guide.md
└── mister/
    ├── rholang_fsm.qsf
    ├── rholang_fsm.qpf
    └── rholang_fsm.sh
```

## Phase 3: FSM Component Implementation

### Implementing FSM Processing Unit

The FSM Processing Unit is the core component that executes the FSM logic:

1. **Create State Definitions**

```verilog
// File: rtl/fsm_processing_unit.v
module fsm_processing_unit (
    input wire clk,
    input wire reset,
    // Event interface
    input wire [7:0] event_in,
    input wire event_valid,
    output reg event_ready,
    // Other interfaces
    // ...
);

    // State encoding
    localparam STATE_INITIAL      = 4'b0000;
    localparam STATE_EVALUATING   = 4'b0001;
    localparam STATE_SENDING      = 4'b0010;
    localparam STATE_RECEIVING    = 4'b0011;
    localparam STATE_WAITING      = 4'b0100;
    localparam STATE_BRANCHING    = 4'b0101;
    localparam STATE_FORKING      = 4'b0110;
    localparam STATE_JOINING      = 4'b0111;
    localparam STATE_BINDING      = 4'b1000;
    localparam STATE_MATCHING     = 4'b1001;
    localparam STATE_CONSTRUCTING = 4'b1010;
    localparam STATE_TERMINATED   = 4'b1111;

    // Event encoding
    localparam EVENT_MESSAGE_AVAILABLE    = 8'h01;
    localparam EVENT_CONDITION_MET        = 8'h02;
    localparam EVENT_EXPRESSION_EVALUATED = 8'h03;
    localparam EVENT_PATTERN_MATCHED      = 8'h04;
    localparam EVENT_TIMEOUT              = 8'h05;
    localparam EVENT_ERROR                = 8'h06;

    // Current state register
    reg [3:0] current_state;
    reg [3:0] next_state;

    // Implement state transition logic
    always @(*) begin
        // Default: stay in current state
        next_state = current_state;

        case (current_state)
            STATE_INITIAL: begin
                // Transitions from INITIAL state
                case (event_in)
                    // Implement state transitions based on events
                    // ...
                endcase
            end

            STATE_EVALUATING: begin
                // Transitions from EVALUATING state
                // ...
            end

            // Implement all other state transitions
            // ...

            STATE_TERMINATED: begin
                // Terminal state - no transitions out
                next_state = STATE_TERMINATED;
            end

            default: begin
                // Invalid state - reset to INITIAL
                next_state = STATE_INITIAL;
            end
        endcase
    end

    // State register update
    always @(posedge clk or posedge reset) begin
        if (reset)
            current_state <= STATE_INITIAL;
        else
            current_state <= next_state;
    end

    // Additional FSM logic
    // ...

endmodule
```

2. **Implement Event Queue**

```verilog
// Add to fsm_processing_unit.v

// Event queue - FIFO for incoming events
reg [7:0] event_queue [15:0];
reg [3:0] queue_head, queue_tail;
wire queue_empty = (queue_head == queue_tail);
wire queue_full = ((queue_tail + 1) & 4'hF) == queue_head;

// Event queue management
always @(posedge clk or posedge reset) begin
    if (reset) begin
        queue_head <= 4'h0;
        queue_tail <= 4'h0;
        event_ready <= 1'b1;
    end else begin
        // Accept new events when queue isn't full
        event_ready <= !queue_full;

        // Add incoming events to queue
        if (event_valid && !queue_full) begin
            event_queue[queue_tail] <= event_in;
            queue_tail <= (queue_tail + 1) & 4'hF;
        end

        // Remove processed events
        if (!queue_empty && event_processed) begin
            queue_head <= (queue_head + 1) & 4'hF;
        end
    end
 end
```

### Implementing Channel Communication Network

The Channel Communication Network handles message passing between FSMs:

```verilog
// File: rtl/channel_communication.v
module channel_communication_network #(
    parameter NUM_FPUS = 16,
    parameter DATA_WIDTH = 32
) (
    input wire clk,
    input wire reset,

    // FPU interfaces (simplified)
    input wire [7:0] fpu_channel_id [NUM_FPUS-1:0],
    input wire [DATA_WIDTH-1:0] fpu_message [NUM_FPUS-1:0],
    input wire fpu_send_valid [NUM_FPUS-1:0],
    output wire fpu_send_ready [NUM_FPUS-1:0],
    output wire [7:0] fpu_recv_channel_id [NUM_FPUS-1:0],
    output wire [DATA_WIDTH-1:0] fpu_recv_message [NUM_FPUS-1:0],
    output wire fpu_recv_valid [NUM_FPUS-1:0],
    input wire fpu_recv_ready [NUM_FPUS-1:0]
);

    // Channel table implementation
    // Channel registration logic
    // Message routing logic
    // ...

endmodule
```

### Implementing Process Creation and Management

```verilog
// File: rtl/process_management.v
module process_management_unit #(
    parameter NUM_FPUS = 16,
    parameter FPU_ID_WIDTH = 4
) (
    input wire clk,
    input wire reset,

    // Process creation interface
    input wire [FPU_ID_WIDTH-1:0] fork_request_from,
    input wire [7:0] num_processes,
    output reg fork_grant,
    output reg [FPU_ID_WIDTH-1:0] new_process_ids [7:0],

    // Process termination interface
    input wire [FPU_ID_WIDTH-1:0] terminate_process_id,
    input wire terminate_valid
);

    // Process allocation table
    reg [1:0] fpu_status [NUM_FPUS-1:0]; // 0=free, 1=allocated, 2=terminating

    // Process management logic
    // ...

endmodule
```

### Implementing Memory Management Unit

```verilog
// File: rtl/memory_management.v
module memory_management_unit (
    input wire clk,
    input wire reset,

    // Memory allocation interface
    input wire [3:0] alloc_request_from,
    input wire [15:0] alloc_size,
    output reg [31:0] alloc_address,
    output reg alloc_grant,

    // Memory deallocation interface
    input wire [31:0] dealloc_address,
    input wire dealloc_valid,

    // Memory access interface
    input wire [31:0] mem_addr,
    input wire mem_write,
    input wire [31:0] mem_write_data,
    output reg [31:0] mem_read_data
);

    // Memory management logic
    // ...

endmodule
```

### Implementing Linux Interface Controller

```verilog
// File: rtl/linux_interface.v
module linux_interface_controller (
    input wire clk,
    input wire reset,

    // HPS interface
    input wire [31:0] hps_writedata,
    output reg [31:0] hps_readdata,
    input wire [7:0] hps_address,
    input wire hps_write,
    input wire hps_read,
    output wire hps_waitrequest,

    // Core control interface
    output reg [31:0] program_data,
    output reg program_valid,
    input wire program_ready,
    output reg start_execution,
    input wire execution_done
);

    // Command and status registers
    reg [31:0] control_reg;
    reg [31:0] status_reg;

    // Linux interface logic
    // ...

endmodule
```

### Top-Level Module Integration

```verilog
// File: rtl/rholang_fsm_top.v
module rholang_fsm_top (
    // Clock and reset
    input wire clk,
    input wire reset,

    // HPS interface
    input wire [31:0] hps_writedata,
    output wire [31:0] hps_readdata,
    input wire [7:0] hps_address,
    input wire hps_write,
    input wire hps_read,
    output wire hps_waitrequest,

    // Debug interface
    output wire [7:0] debug_leds
);

    // Internal signals
    wire [31:0] program_data;
    wire program_valid;
    wire program_ready;
    wire start_execution;
    wire execution_done;

    // Instantiate all components and connect them
    // ...

endmodule
```

## Phase 4: Integration with MiSTer

### Creating MiSTer Core Files

1. **Create Quartus Project File**

```
# File: mister/rholang_fsm.qsf
set_global_assignment -name FAMILY "Cyclone V"
set_global_assignment -name DEVICE 5CSEBA6U23I7
set_global_assignment -name TOP_LEVEL_ENTITY sys_top
set_global_assignment -name ORIGINAL_QUARTUS_VERSION 17.0.0
set_global_assignment -name PROJECT_CREATION_TIME_DATE "2025-07-01 10:00:00"
set_global_assignment -name LAST_QUARTUS_VERSION "17.0.0 Lite Edition"

# Include standard MiSTer constraints
set_global_assignment -name VERILOG_FILE rtl/rholang_fsm_top.v
set_global_assignment -name VERILOG_FILE rtl/fsm_processing_unit.v
set_global_assignment -name VERILOG_FILE rtl/channel_communication.v
set_global_assignment -name VERILOG_FILE rtl/process_management.v
set_global_assignment -name VERILOG_FILE rtl/memory_management.v
set_global_assignment -name VERILOG_FILE rtl/linux_interface.v

# Include standard MiSTer constraints
# ...
```

2. **Create Core Launcher Script**

```bash
# File: mister/rholang_fsm.sh
#!/bin/bash

# Navigate to script directory
cd "$(dirname "$0")"

# Create required directories
mkdir -p /media/fat/rholang

# Load Rholang FSM core
load_core rholang_fsm
```

3. **Create MiSTer Menu Integration**

```ini
# This will be part of the MiSTer menu system
[Rholang FSM]
rbf=rholang_fsm.rbf
```

## Phase 5: Tools for Rholang Programs

### Implementing Rholang Parser and FSM Encoder

1. **Create Parser Tool**

```cpp
// File: tools/rholang_parser.cpp
#include <iostream>
#include <fstream>
#include <string>
#include <vector>

// Define FSM encoding structures
struct FSMState {
    uint8_t state_id;
    uint8_t state_type;
    // Other state data
};

struct FSMTransition {
    uint8_t from_state;
    uint8_t to_state;
    uint8_t event_type;
    // Other transition data
};

struct FSMProgram {
    std::vector<FSMState> states;
    std::vector<FSMTransition> transitions;
    // Other program data
};

// Parse Rholang code to FSM representation
FSMProgram parseRholang(const std::string& source) {
    FSMProgram program;

    // Implement parsing logic here
    // This would typically use a parser generator like ANTLR
    // For complex grammars like Rholang

    return program;
}

// Serialize FSM to binary format for FPGA consumption
void serializeFSM(const FSMProgram& program, std::ostream& output) {
    // Write header
    uint32_t magic = 0x52484F46; // 'RHOF'
    output.write(reinterpret_cast<char*>(&magic), sizeof(magic));

    // Write state count
    uint32_t stateCount = program.states.size();
    output.write(reinterpret_cast<char*>(&stateCount), sizeof(stateCount));

    // Write states
    for (const auto& state : program.states) {
        output.write(reinterpret_cast<const char*>(&state), sizeof(state));
    }

    // Write transition count
    uint32_t transitionCount = program.transitions.size();
    output.write(reinterpret_cast<char*>(&transitionCount), sizeof(transitionCount));

    // Write transitions
    for (const auto& transition : program.transitions) {
        output.write(reinterpret_cast<const char*>(&transition), sizeof(transition));
    }

    // Write other program data
    // ...
}

int main(int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "Usage: " << argv[0] << " <input.rho> <output.fsm>" << std::endl;
        return 1;
    }

    // Read input file
    std::ifstream inFile(argv[1]);
    if (!inFile) {
        std::cerr << "Error: Could not open input file" << std::endl;
        return 1;
    }

    std::string source((std::istreambuf_iterator<char>(inFile)), std::istreambuf_iterator<char>());

    // Parse Rholang to FSM
    FSMProgram program = parseRholang(source);

    // Write binary output
    std::ofstream outFile(argv[2], std::ios::binary);
    if (!outFile) {
        std::cerr << "Error: Could not open output file" << std::endl;
        return 1;
    }

    serializeFSM(program, outFile);

    std::cout << "Successfully compiled " << argv[1] << " to " << argv[2] << std::endl;
    std::cout << "States: " << program.states.size() << ", Transitions: " << program.transitions.size() << std::endl;

    return 0;
}
```

## Phase 6: Testing and Validation

### Testbench for FSM Execution

```verilog
// File: sim/testbench.v
module testbench;
    // Clock and reset
    reg clk;
    reg reset;

    // Test signals
    reg [31:0] hps_writedata;
    wire [31:0] hps_readdata;
    reg [7:0] hps_address;
    reg hps_write;
    reg hps_read;
    wire hps_waitrequest;

    // Debug output
    wire [7:0] debug_leds;

    // Instantiate DUT
    rholang_fsm_top dut (
        .clk(clk),
        .reset(reset),
        .hps_writedata(hps_writedata),
        .hps_readdata(hps_readdata),
        .hps_address(hps_address),
        .hps_write(hps_write),
        .hps_read(hps_read),
        .hps_waitrequest(hps_waitrequest),
        .debug_leds(debug_leds)
    );

    // Clock generation
    initial begin
        clk = 0;
        forever #5 clk = ~clk;
    end

    // Test sequence
    initial begin
        // Initialize
        reset = 1;
        hps_write = 0;
        hps_read = 0;
        hps_address = 0;
        hps_writedata = 0;

        // Apply reset
        #100;
        reset = 0;
        #100;

        // Load program data
        // ...

        // Start execution
        // ...

        // Monitor execution
        // ...

        // End simulation
        #10000;
        $finish;
    end

    // Monitor for checking outputs
    // ...

endmodule
```

### Sample Test Programs

1. **Simple Message Passing**

```rholang
// File: sim/test_programs/message_passing.rho
new channel in {
  channel!("Hello, Rholang") |
  for (msg <- channel) {
    new stdout(`rho:io:stdout`) in {
      stdout!(msg)
    }
  }
}
```

2. **Pattern Matching**

```rholang
// File: sim/test_programs/pattern_matching.rho
new channel, stdout(`rho:io:stdout`) in {
  channel!([1, 2, 3, 4, 5]) |
  for (list <- channel) {
    match list {
      [1, 2, ...rest] => {
        stdout!("Pattern matched: " ++ rest.length().toInteger().toString())
      }
      _ => {
        stdout!("Pattern did not match")
      }
    }
  }
}
```

## Phase 7: Optimization

After initial implementation, apply these optimizations:

1. **Resource Optimization**
   - Profile resource usage with Quartus tools
   - Identify critical paths and bottlenecks
   - Optimize FSM encoding for minimum logic usage

2. **Performance Optimization**
   - Implement pipelining for FSM operations
   - Optimize channel communication network
   - Reduce memory access latency

3. **Scalability Optimization**
   - Make number of FSM units configurable
   - Implement dynamic resource allocation
   - Optimize for varying program sizes

## Phase 8: Documentation and Deployment

### User Guide

Create comprehensive documentation:

1. **Installation Guide**
   - How to install the core on MiSTer
   - Required hardware and software

2. **Programming Guide**
   - How to write and compile Rholang programs
   - FSM model explanation

3. **Operation Guide**
   - How to load and run programs
   - Using debugging features

4. **Performance Metrics**
   - Resource utilization statistics
   - Performance benchmarks

### Deployment Package

Prepare final deployment package:

1. **Core Files**
   - Compiled rbf file
   - Launcher script

2. **Tools**
   - Compiled rholang_parser binary
   - Supporting scripts

3. **Documentation**
   - User guide
   - Technical documentation

4. **Example Programs**
   - Sample Rholang programs
   - Demonstration scripts

## Conclusion

By following this implementation guide, you can create a fully functional Rholang FSM core for the MiSTer FPGA platform. This implementation leverages the formal FSM model of Rholang to provide hardware acceleration for concurrent, message-passing programs.

The modular design allows for future enhancements and optimizations, while the comprehensive testing strategy ensures correctness and performance. The integration with MiSTer's Linux-based ecosystem provides a user-friendly interface for loading and running Rholang programs on FPGA hardware.

## References

1. MiSTer FPGA Project: https://github.com/MiSTer-devel/Main_MiSTer
2. Rholang Language: https://rholang.github.io/docs/rholang/
3. Finite State Machines in Verilog: https://www.digikey.com/en/maker/projects/introduction-to-fpga-part-5-finite-state-machine-fsm/4d83e63da76044af9acc8aa7dcf07c22
4. Process Calculi and Concurrency Models: https://en.wikipedia.org/wiki/Process_calculus
