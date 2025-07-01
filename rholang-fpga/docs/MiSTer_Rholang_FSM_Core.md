# Rholang FSM Implementation for MiSTer FPGA Platform

## Introduction

This document provides comprehensive guidance for implementing a Rholang Finite State Machine (FSM) as an FPGA core for the MiSTer platform. The MiSTer FPGA is an open-source project that provides hardware reimplementation of various computing systems using Field Programmable Gate Arrays (FPGAs). By implementing Rholang's FSM model in MiSTer, we create a hardware accelerator for executing Rholang programs with high determinism and performance.

## Project Overview

Rholang is a concurrent, message-passing programming language designed for distributed systems. Its execution model is based on the π-calculus and incorporates functional programming principles. This project creates a hardware implementation of the Rholang FSM execution model as described in the language specification, providing:

1. Hardware acceleration for Rholang programs
2. Deterministic execution of concurrent processes
3. Integration with the MiSTer FPGA Linux-based ecosystem
4. A foundation for formal verification of Rholang programs

## MiSTer FPGA Architecture Overview

The MiSTer platform consists of:

1. **DE10-Nano Board** - The hardware foundation with a Cyclone V FPGA
2. **Linux OS** - A Linux-based operating system that manages cores and provides user interface
3. **FPGA Cores** - Hardware implementations of various systems loaded into the FPGA
4. **Add-on Boards** - Optional hardware expansions for additional functionality

MiSTer cores operate by having the Linux system load an FPGA configuration (bitstream) that implements a specific hardware design. For our Rholang FSM implementation, we'll create a core that implements the finite state machine execution model described in the Rholang specification.

## Rholang FSM Model Overview

The Rholang FSM model represents program execution through states and transitions:

### Key States

- **INITIAL** - Starting state for any process
- **EVALUATING** - Evaluating an expression (functional evaluation)
- **SENDING** - Sending a message (concurrent communication)
- **RECEIVING** - Receiving a message (concurrent communication)
- **WAITING** - Waiting for a condition (synchronization primitive)
- **BRANCHING** - Making a decision (functional control flow)
- **FORKING** - Creating parallel processes (concurrent execution)
- **JOINING** - Synchronizing parallel processes (concurrent coordination)
- **BINDING** - Binding variables (lambda calculus substitution)
- **MATCHING** - Pattern matching (functional decomposition)
- **CONSTRUCTING** - Constructing data structures (functional composition)
- **TERMINATED** - Process has terminated (final state)

### Key Transitions

- **EVALUATE** - Evaluate an expression
- **SEND** - Send a message
- **RECEIVE** - Receive a message
- **FORK** - Create parallel processes
- **JOIN** - Synchronize parallel processes
- **BIND** - Bind a variable
- **MATCH** - Perform pattern matching
- **BRANCH** - Make a conditional decision
- **CONSTRUCT** - Construct a data structure
- **TERMINATE** - Terminate a process

### Events

- **MESSAGE_AVAILABLE** - A message is available on a channel
- **CONDITION_MET** - A condition has been satisfied
- **EXPRESSION_EVALUATED** - An expression has been evaluated
- **PATTERN_MATCHED** - A pattern has been matched
- **TIMEOUT** - A timeout has occurred
- **ERROR** - An error has occurred

## Hardware Implementation Strategy

### System Architecture

Our Rholang FSM core for MiSTer will consist of the following components:

1. **FSM Processing Units (FPUs)** - Hardware modules that implement FSM execution
2. **Channel Communication Network (CCN)** - Network for inter-FSM message passing
3. **Process Creation and Management Unit (PCMU)** - Manages process lifecycle
4. **Memory Management Unit (MMU)** - Manages memory for processes and channels
5. **Linux Interface Controller (LIC)** - Interface with the MiSTer Linux OS

![System Architecture Diagram](https://example.com/architecture.png)

### FSM Processing Units (FPUs)

Each FPU implements the execution of a single FSM instance and contains:

1. **State Register** - Stores current state of the FSM
2. **Event Queue** - FIFO queue for pending events
3. **Transition Logic** - Combinational logic for state transitions
4. **Local Storage** - Storage for FSM-specific data

```verilog
module FSM_Processing_Unit (
    input wire clk,
    input wire reset,
    input wire [7:0] event_in,
    input wire event_valid,
    output wire event_ready,
    output wire [7:0] event_out,
    output wire event_out_valid,
    input wire event_out_ready,
    // Other interfaces
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

    // Event queue - simplified for illustration
    reg [7:0] event_queue [15:0];
    reg [3:0] queue_head, queue_tail;
    wire queue_empty = (queue_head == queue_tail);
    wire queue_full = ((queue_tail + 1) & 4'hF) == queue_head;

    // State transition logic
    always @(*) begin
        next_state = current_state; // Default: stay in current state

        if (!queue_empty) begin
            case (current_state)
                STATE_INITIAL: begin
                    // Transition logic for INITIAL state
                    case (event_queue[queue_head])
                        // Handle events...
                    endcase
                end

                STATE_EVALUATING: begin
                    // Transition logic for EVALUATING state
                    case (event_queue[queue_head])
                        EVENT_EXPRESSION_EVALUATED: next_state = STATE_SENDING;
                        // Handle other events...
                    endcase
                end

                // Other states...
            endcase
        end
    end

    // State register update
    always @(posedge clk or posedge reset) begin
        if (reset)
            current_state <= STATE_INITIAL;
        else
            current_state <= next_state;
    end

    // Event queue management
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            queue_head <= 4'h0;
            queue_tail <= 4'h0;
        end else begin
            // Add incoming events to queue
            if (event_valid && !queue_full) begin
                event_queue[queue_tail] <= event_in;
                queue_tail <= (queue_tail + 1) & 4'hF;
            end

            // Remove processed events
            if (!queue_empty && /* event processed condition */) begin
                queue_head <= (queue_head + 1) & 4'hF;
            end
        end
    end

    // Other logic...

endmodule
```

### Channel Communication Network (CCN)

The CCN implements message passing between FSMs:

```verilog
module Channel_Communication_Network (
    input wire clk,
    input wire reset,

    // FPU interfaces (multiple ports, simplified here)
    input wire [7:0] fpu_channel_id [NUM_FPUS-1:0],
    input wire [DATA_WIDTH-1:0] fpu_message [NUM_FPUS-1:0],
    input wire fpu_send_valid [NUM_FPUS-1:0],
    output wire fpu_send_ready [NUM_FPUS-1:0],
    output wire [7:0] fpu_recv_channel_id [NUM_FPUS-1:0],
    output wire [DATA_WIDTH-1:0] fpu_recv_message [NUM_FPUS-1:0],
    output wire fpu_recv_valid [NUM_FPUS-1:0],
    input wire fpu_recv_ready [NUM_FPUS-1:0]
);

    // Channel table - maps channel IDs to receiving FPUs
    reg [FPU_ID_WIDTH-1:0] channel_table [255:0][MAX_RECEIVERS-1:0];
    reg [3:0] receiver_count [255:0]; // Count of receivers for each channel

    // Message routing logic
    genvar i, j;
    generate
        for (i = 0; i < NUM_FPUS; i = i + 1) begin: send_logic
            always @(posedge clk) begin
                if (fpu_send_valid[i] && fpu_send_ready[i]) begin
                    // Route message to all receivers of this channel
                    for (int j = 0; j < receiver_count[fpu_channel_id[i]]; j = j + 1) begin
                        // Queue message to appropriate FPU's receive queue
                        // ...
                    end
                end
            end
        end
    endgenerate

    // Channel registration logic
    // ...

endmodule
```

### Process Creation and Management Unit (PCMU)

The PCMU handles creation and termination of FSM instances:

```verilog
module Process_Creation_Management_Unit (
    input wire clk,
    input wire reset,

    // FPU control interfaces
    input wire [FPU_ID_WIDTH-1:0] fork_request_from,
    input wire [7:0] num_processes,
    output wire fork_grant,
    output wire [FPU_ID_WIDTH-1:0] new_process_ids [MAX_FORK-1:0],

    // Process termination interface
    input wire [FPU_ID_WIDTH-1:0] terminate_process_id,
    input wire terminate_valid,

    // Other interfaces
);

    // Process allocation table
    reg [1:0] fpu_status [NUM_FPUS-1:0]; // 0=free, 1=allocated, 2=terminating

    // Free FPU queue
    reg [FPU_ID_WIDTH-1:0] free_queue [NUM_FPUS-1:0];
    reg [FPU_ID_WIDTH-1:0] free_head, free_tail;
    wire free_queue_empty = (free_head == free_tail);

    // Fork request handling
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            // Initialize all FPUs as free
            for (int i = 0; i < NUM_FPUS; i = i + 1) begin
                fpu_status[i] <= 2'b00;
                free_queue[i] <= i;
            end
            free_head <= 0;
            free_tail <= NUM_FPUS;
            fork_grant <= 1'b0;
        end else begin
            // Handle fork requests
            if (fork_request_from != {FPU_ID_WIDTH{1'b1}} && !free_queue_empty) begin
                // Check if we have enough free FPUs
                if (free_tail - free_head >= num_processes) begin
                    fork_grant <= 1'b1;

                    // Allocate FPUs
                    for (int i = 0; i < num_processes; i = i + 1) begin
                        new_process_ids[i] <= free_queue[free_head + i];
                        fpu_status[free_queue[free_head + i]] <= 2'b01; // Mark as allocated
                    end

                    // Update free queue
                    free_head <= free_head + num_processes;
                end
            end else begin
                fork_grant <= 1'b0;
            end

            // Handle termination
            if (terminate_valid) begin
                fpu_status[terminate_process_id] <= 2'b00; // Mark as free
                free_queue[free_tail] <= terminate_process_id;
                free_tail <= free_tail + 1;
            end
        end
    end

endmodule
```

### Memory Management Unit (MMU)

The MMU manages memory allocation for FSM instances:

```verilog
module Memory_Management_Unit (
    input wire clk,
    input wire reset,

    // Memory allocation interface
    input wire [FPU_ID_WIDTH-1:0] alloc_request_from,
    input wire [15:0] alloc_size,
    output wire [31:0] alloc_address,
    output wire alloc_grant,

    // Memory deallocation interface
    input wire [31:0] dealloc_address,
    input wire dealloc_valid,

    // Memory access interface
    input wire [31:0] mem_addr,
    input wire mem_write,
    input wire [31:0] mem_write_data,
    output wire [31:0] mem_read_data,

    // SDRAM interface
    // ...
);

    // Memory allocation logic using free lists
    // ...

    // Memory access logic
    // ...

endmodule
```

### Linux Interface Controller (LIC)

The LIC provides the interface between the MiSTer Linux OS and the Rholang FSM core:

```verilog
module Linux_Interface_Controller (
    input wire clk,
    input wire reset,

    // HPS interface
    input wire [31:0] hps_writedata,
    output wire [31:0] hps_readdata,
    input wire [7:0] hps_address,
    input wire hps_write,
    input wire hps_read,
    output wire hps_waitrequest,

    // FSM control interface
    output wire [31:0] program_data,
    output wire program_valid,
    input wire program_ready,
    output wire start_execution,
    input wire execution_done,

    // Status and debug interface
    output wire [31:0] status_data,
    input wire [7:0] status_address,
    input wire status_read,
    output wire status_valid
);

    // Command registers
    reg [31:0] control_reg;
    reg [31:0] status_reg;
    reg [31:0] program_counter;
    reg [31:0] program_memory [1023:0]; // Buffer for program data

    // Interface logic
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            control_reg <= 32'h0;
            program_counter <= 32'h0;
            program_valid <= 1'b0;
            start_execution <= 1'b0;
        end else begin
            // Handle HPS writes
            if (hps_write) begin
                case (hps_address)
                    8'h00: control_reg <= hps_writedata;
                    8'h04: begin // Program data port
                        program_memory[program_counter] <= hps_writedata;
                        program_counter <= program_counter + 1;
                    end
                    // Other registers
                endcase
            end

            // Control register bits
            start_execution <= control_reg[0];

            // Program data streaming
            if (control_reg[1] && program_counter > 0) begin
                program_valid <= 1'b1;
                if (program_ready) begin
                    program_counter <= program_counter - 1;
                    program_data <= program_memory[program_counter - 1];
                    if (program_counter == 1) begin
                        control_reg[1] <= 1'b0; // Clear program load bit
                    end
                end
            end else begin
                program_valid <= 1'b0;
            end

            // Status updates
            if (execution_done) begin
                status_reg[0] <= 1'b1; // Set execution done bit
            end
        end
    end

    // HPS read logic
    always @(*) begin
        if (hps_read) begin
            case (hps_address)
                8'h00: hps_readdata = control_reg;
                8'h08: hps_readdata = status_reg;
                8'h0C: hps_readdata = program_counter;
                // Other registers
                default: hps_readdata = 32'h0;
            endcase
        end else begin
            hps_readdata = 32'h0;
        end
    end

endmodule
```

## Top-Level Integration

The top-level module integrates all components:

```verilog
module Rholang_FSM_Core (
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

    // SDRAM interface
    // ...

    // Debug interface
    output wire [7:0] debug_leds
);

    // Internal signals
    wire [31:0] program_data;
    wire program_valid;
    wire program_ready;
    wire start_execution;
    wire execution_done;

    // Instantiate Linux Interface Controller
    Linux_Interface_Controller lic (
        .clk(clk),
        .reset(reset),
        .hps_writedata(hps_writedata),
        .hps_readdata(hps_readdata),
        .hps_address(hps_address),
        .hps_write(hps_write),
        .hps_read(hps_read),
        .hps_waitrequest(hps_waitrequest),
        .program_data(program_data),
        .program_valid(program_valid),
        .program_ready(program_ready),
        .start_execution(start_execution),
        .execution_done(execution_done),
        // Other connections
    );

    // Instantiate Process Creation and Management Unit
    // ...

    // Instantiate Memory Management Unit
    // ...

    // Instantiate Channel Communication Network
    // ...

    // Instantiate FSM Processing Units
    // ...

    // Debug outputs
    assign debug_leds = {execution_done, start_execution, program_valid, program_ready, 4'h0};

endmodule
```

## Implementation Steps

### 1. Development Environment Setup

1. **Install MiSTer Development Tools**
   - Set up the MiSTer development environment
   - Install Quartus Prime for FPGA synthesis
   - Clone the MiSTer repository

   ```bash
   git clone https://github.com/MiSTer-devel/Main_MiSTer.git
   cd Main_MiSTer
   ```

2. **Create Core Directory Structure**
   ```bash
   mkdir -p cores/rholang_fsm
   cd cores/rholang_fsm
   ```

### 2. Rholang FSM Parser Implementation

Create a parser for Rholang programs that converts them to an FSM representation:

1. **Define FSM Encoding Format**
   - Create a binary format that represents FSM states, transitions, and events
   - Implement serialization/deserialization functions

2. **Implement Parser in C++**
   - Implement a parser that converts Rholang source code to FSM encoding
   - Integrate with MiSTer's Linux OS

   ```cpp
   // rholang_parser.cpp
   #include <iostream>
   #include <fstream>
   #include <string>
   #include <vector>

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

   FSMProgram parseRholang(const std::string& source) {
       FSMProgram program;

       // Parsing logic here
       // ...

       return program;
   }

   int main(int argc, char** argv) {
       if (argc != 3) {
           std::cerr << "Usage: " << argv[0] << " <input.rho> <output.fsm>" << std::endl;
           return 1;
       }

       // Read input file
       std::ifstream inFile(argv[1]);
       std::string source((std::istreambuf_iterator<char>(inFile)), std::istreambuf_iterator<char>());

       // Parse Rholang to FSM
       FSMProgram program = parseRholang(source);

       // Write binary output
       std::ofstream outFile(argv[2], std::ios::binary);
       // Serialization logic
       // ...

       return 0;
   }
   ```

### 3. Core Implementation

1. **Implement FSM Processing Units**
   - Create Verilog modules for FSM execution
   - Implement state transition logic
   - Implement event handling

2. **Implement Communication Network**
   - Create channel-based message passing system
   - Implement message routing logic

3. **Implement Process Management**
   - Create process creation and termination logic
   - Implement process scheduling

4. **Implement Memory Management**
   - Create memory allocation system
   - Implement garbage collection

5. **Implement Linux Interface**
   - Create interface for loading programs
   - Implement status reporting
   - Create debugging interfaces

### 4. Core Integration with MiSTer

1. **Create MiSTer Core Files**
   - Implement required MiSTer interfaces
   - Create configuration files

   ```
   # rholang_fsm.qsf
   set_global_assignment -name FAMILY "Cyclone V"
   set_global_assignment -name DEVICE 5CSEBA6U23I7
   set_global_assignment -name TOP_LEVEL_ENTITY sys_top
   set_global_assignment -name ORIGINAL_QUARTUS_VERSION 17.0.0
   set_global_assignment -name PROJECT_CREATION_TIME_DATE "2025-07-01 10:00:00"
   set_global_assignment -name LAST_QUARTUS_VERSION "17.0.0 Lite Edition"
   # ... additional configuration
   ```

2. **Create Core Launcher Script**
   ```bash
   #!/bin/bash
   # /media/fat/rholang_fsm.sh

   # Navigate to script directory
   cd "$(dirname "$0")"

   # Load Rholang FSM core
   load_core rholang_fsm
   ```

3. **Create Core Menu Integration**
   ```ini
   # /media/fat/menu.rbf
   [Rholang FSM]
   rbf=rholang_fsm.rbf
   ```

### 5. Testing and Debugging

1. **Create Test Programs**
   - Implement simple Rholang programs for testing
   - Create test vectors for FSM transitions

2. **Implement Debugging Interfaces**
   - Create debug output registers
   - Implement waveform capture points
   - Create visualization tools

3. **Validation Process**
   - Test each FSM state and transition
   - Test communication between FSMs
   - Test integration with MiSTer Linux OS

## Execution of Rholang Programs

### Program Loading Process

1. **Compile Rholang Program**
   ```bash
   ./rholang_parser program.rho program.fsm
   ```

2. **Copy to MiSTer**
   ```bash
   scp program.fsm root@mister:/media/fat/rholang/
   ```

3. **Load Core and Program**
   - Start MiSTer and select Rholang FSM core
   - Use OSD menu to load program file
   - Start execution

### Monitoring and Debugging

1. **Status Display**
   - Core shows current execution state on OSD
   - LEDs indicate overall status

2. **Debug Interface**
   - Connect to debug port for detailed execution trace
   - View channel contents and message queues

3. **Error Handling**
   - System reports execution errors
   - Provides error codes and context information

## Rholang FSM Example: Parallel Processes

Here's an example of a simple Rholang program and its FSM representation:

### Rholang Code
```rholang
// Simple parallel processes with message passing
new channel in {
  // Process 1: Send a message
  channel!("Hello, Rholang") |

  // Process 2: Receive the message
  for (msg <- channel) {
    new stdout(`rho:io:stdout`) in {
      stdout!(msg)
    }
  }
}
```

### FSM Representation

This would be represented as multiple interconnected FSMs:

1. **Main Process FSM**
   - INITIAL → BINDING (channel) → FORKING → JOINING → TERMINATED

2. **Sender Process FSM**
   - INITIAL → EVALUATING (channel) → EVALUATING ("Hello, Rholang") → SENDING → TERMINATED

3. **Receiver Process FSM**
   - INITIAL → EVALUATING (channel) → RECEIVING → BINDING (msg) → BINDING (stdout) → EVALUATING (msg) → SENDING → TERMINATED

### Hardware Execution

In the FPGA implementation:

1. The Main Process FSM allocates a channel and forks two child processes
2. The Sender Process FSM evaluates its message and sends it on the channel
3. The Channel Communication Network routes the message to the Receiver Process FSM
4. The Receiver Process FSM receives the message, binds it to a variable, and sends it to stdout
5. The Process Creation and Management Unit tracks process completion
6. The Main Process FSM joins the child processes and terminates

## Performance Considerations

### Parallelism

The FPGA implementation provides true hardware parallelism for Rholang processes:

1. **Multiple FSM Processing Units** - Execute different processes in parallel
2. **Pipelined Execution** - Pipeline stages for different FSM operations
3. **Concurrent Message Passing** - Hardware message routing network

### Resource Utilization

Resource considerations for the MiSTer FPGA:

1. **Logic Elements** - FSM transition logic requires significant resources
2. **Memory** - Channel buffers and process state storage
3. **DSP Blocks** - For arithmetic operations in Rholang expressions

### Optimizations

Potential optimizations for the implementation:

1. **FSM Compression** - Reduce state encoding size
2. **Event Prioritization** - Prioritize critical events
3. **Memory Hierarchy** - Implement caching for process state
4. **Dynamic Resource Allocation** - Allocate hardware resources based on program needs

## Conclusion

Implementing a Rholang FSM core for MiSTer FPGA provides a unique hardware platform for executing concurrent, message-passing programs with deterministic behavior. This implementation demonstrates how formal computational models can be directly realized in hardware, providing benefits in performance, determinism, and formal verification.

The modular design allows for future enhancements, such as:

1. **Integration with Other Cores** - Allow Rholang programs to control other MiSTer cores
2. **Extended Instruction Set** - Add hardware acceleration for specific Rholang operations
3. **Formal Verification** - Use the FSM model for formal verification of programs
4. **Visual Debugging** - Create visual tools for FSM execution monitoring

By following this implementation guide, you can create a powerful hardware accelerator for Rholang programs that leverages the MiSTer FPGA platform's capabilities while maintaining compatibility with its Linux-based ecosystem.

## References

1. MiSTer FPGA Project: https://github.com/MiSTer-devel/Main_MiSTer
2. Rholang Language Specification: https://rholang.github.io/docs/rholang/
3. Finite State Machines in FPGAs: https://www.digikey.com/en/maker/projects/introduction-to-fpga-part-5-finite-state-machine-fsm/4d83e63da76044af9acc8aa7dcf07c22
4. Process Calculi for Programming Languages: https://en.wikipedia.org/wiki/Process_calculus
