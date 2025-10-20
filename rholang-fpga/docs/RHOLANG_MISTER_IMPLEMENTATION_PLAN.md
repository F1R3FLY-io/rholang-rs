# Rholang FSM Implementation for MiSTer FPGA Platform - Implementation Plan

## 1. System Architecture

### 1.1 Overall System Architecture

The Rholang FSM core for MiSTer FPGA consists of five main components that work together to execute Rholang programs in hardware:

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rholang FSM Core                           │
│                                                                 │
│  ┌─────────────┐       ┌─────────────┐       ┌─────────────┐   │
│  │    Linux    │       │   Process   │       │    Memory   │   │
│  │  Interface  │◄────►│  Creation &  │◄────►│  Management  │   │
│  │ Controller  │       │ Management  │       │     Unit    │   │
│  └──────┬──────┘       └──────┬──────┘       └──────┬──────┘   │
│         │                     │                     │          │
│         │                     │                     │          │
│         ▼                     ▼                     ▼          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                                                         │   │
│  │              Channel Communication Network              │   │
│  │                                                         │   │
│  └─────────────────────────┬───────────────────────────────┘   │
│                            │                                   │
│                            │                                   │
│                            ▼                                   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                                                         │   │
│  │                 FSM Processing Units                    │   │
│  │                                                         │   │
│  │  ┌─────────┐  ┌─────────┐  ┌─────────┐    ┌─────────┐  │   │
│  │  │   FPU   │  │   FPU   │  │   FPU   │... │   FPU   │  │   │
│  │  │    #1   │  │    #2   │  │    #3   │    │    #N   │  │   │
│  │  └─────────┘  └─────────┘  └─────────┘    └─────────┘  │   │
│  │                                                         │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 Component Descriptions

1. **Linux Interface Controller (LIC)**
   - Interfaces with MiSTer's Linux OS
   - Handles program loading and execution control
   - Provides status reporting and debugging interfaces

2. **Process Creation and Management Unit (PCMU)**
   - Manages FSM instance lifecycle (creation, scheduling, termination)
   - Allocates FSM Processing Units to processes
   - Tracks process dependencies and relationships

3. **Memory Management Unit (MMU)**
   - Manages memory allocation for processes and channels
   - Implements garbage collection for terminated processes
   - Provides memory access arbitration

4. **Channel Communication Network (CCN)**
   - Implements message passing between FSM instances
   - Routes messages based on channel identifiers
   - Manages channel registration and deregistration

5. **FSM Processing Units (FPUs)**
   - Execute individual FSM instances
   - Implement state transition logic
   - Process events and generate new events

### 1.3 Data Flow

1. **Program Loading**
   - Linux OS → LIC → MMU (program storage) → PCMU (initial process creation)

2. **Process Execution**
   - PCMU → FPUs (process allocation)
   - FPUs → CCN (message sending)
   - CCN → FPUs (message receiving)
   - FPUs → MMU (memory allocation/access)

3. **Process Creation**
   - FPU → PCMU (fork request) → MMU (memory allocation) → FPUs (new processes)

4. **Process Termination**
   - FPU → PCMU (termination notification) → MMU (memory deallocation)

5. **Status Reporting**
   - FPUs → LIC → Linux OS

## 2. Detailed Component Design

### 2.1 FSM Processing Units (FPUs)

#### 2.1.1 Architecture

Each FPU contains:
- State register (current FSM state)
- Event queue (FIFO for pending events)
- Transition logic (combinational logic for state transitions)
- Local storage (registers for process-specific data)
- Control interface (for PCMU interaction)

#### 2.1.2 State Encoding

```verilog
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
localparam STATE_OPERATING    = 4'b1011;
localparam STATE_TERMINATED   = 4'b1111;
```

#### 2.1.3 Event Encoding

```verilog
// Event encoding
localparam EVENT_MESSAGE_AVAILABLE    = 8'h01;
localparam EVENT_CONDITION_MET        = 8'h02;
localparam EVENT_EXPRESSION_EVALUATED = 8'h03;
localparam EVENT_PATTERN_MATCHED      = 8'h04;
localparam EVENT_PATTERN_NOT_MATCHED  = 8'h05;
localparam EVENT_TIMEOUT              = 8'h06;
localparam EVENT_ERROR                = 8'h07;
localparam EVENT_SIGNAL               = 8'h08;
```

#### 2.1.4 Transition Logic

The core of each FPU is the state transition logic that determines the next state based on the current state and incoming events:

```verilog
// State transition logic
always @(*) begin
    next_state = current_state; // Default: stay in current state
    
    if (!queue_empty) begin
        case (current_state)
            STATE_INITIAL: begin
                // Initial state transitions based on process type
                case (process_type)
                    PROCESS_SEND: next_state = STATE_EVALUATING;
                    PROCESS_RECEIVE: next_state = STATE_EVALUATING;
                    PROCESS_PAR: next_state = STATE_FORKING;
                    // Other process types...
                endcase
            end
            
            STATE_EVALUATING: begin
                if (event_queue[queue_head] == EVENT_EXPRESSION_EVALUATED) begin
                    case (process_type)
                        PROCESS_SEND: next_state = STATE_SENDING;
                        PROCESS_RECEIVE: next_state = STATE_RECEIVING;
                        // Other cases...
                    endcase
                end
            end
            
            STATE_SENDING: begin
                // After sending, transition to terminated
                next_state = STATE_TERMINATED;
            end
            
            STATE_RECEIVING: begin
                if (event_queue[queue_head] == EVENT_MESSAGE_AVAILABLE) begin
                    // After receiving, process the message
                    next_state = STATE_BINDING;
                end
            end
            
            // Other state transitions...
            
            STATE_TERMINATED: begin
                // Terminal state - no transitions out
                next_state = STATE_TERMINATED;
            end
        endcase
    end
end
```

### 2.2 Channel Communication Network (CCN)

#### 2.2.1 Architecture

The CCN consists of:
- Channel table (maps channel IDs to receiving FPUs)
- Message routing logic (routes messages from senders to receivers)
- Channel registration logic (manages channel subscriptions)
- Message queues (buffers messages for each channel)

#### 2.2.2 Channel Table

```verilog
// Channel table structure
reg [FPU_ID_WIDTH-1:0] channel_table [NUM_CHANNELS-1:0][MAX_RECEIVERS-1:0];
reg [3:0] receiver_count [NUM_CHANNELS-1:0]; // Count of receivers for each channel
```

#### 2.2.3 Message Routing

```verilog
// Message routing logic
always @(posedge clk) begin
    if (fpu_send_valid[sending_fpu] && fpu_send_ready[sending_fpu]) begin
        // Get channel ID from sending FPU
        channel_id = fpu_channel_id[sending_fpu];
        
        // Route message to all receivers of this channel
        for (int i = 0; i < receiver_count[channel_id]; i = i + 1) begin
            receiver_fpu = channel_table[channel_id][i];
            
            // Queue message to receiver's event queue
            if (!fpu_event_queue_full[receiver_fpu]) begin
                fpu_event_in[receiver_fpu] <= EVENT_MESSAGE_AVAILABLE;
                fpu_event_data[receiver_fpu] <= fpu_message[sending_fpu];
                fpu_event_valid[receiver_fpu] <= 1'b1;
            end
        end
    end
end
```

### 2.3 Process Creation and Management Unit (PCMU)

#### 2.3.1 Architecture

The PCMU consists of:
- Process allocation table (tracks FPU allocation status)
- Free FPU queue (manages available FPUs)
- Fork request handling logic (allocates FPUs for new processes)
- Process termination logic (reclaims FPUs from terminated processes)
- Process dependency tracking (manages parent-child relationships)

#### 2.3.2 Process Allocation

```verilog
// Process allocation logic
always @(posedge clk or posedge reset) begin
    if (reset) begin
        // Initialize all FPUs as free
        for (int i = 0; i < NUM_FPUS; i = i + 1) begin
            fpu_status[i] <= FPU_FREE;
            free_queue[i] <= i;
        end
        free_head <= 0;
        free_tail <= NUM_FPUS;
        fork_grant <= 1'b0;
    end else begin
        // Handle fork requests
        if (fork_request_valid && !free_queue_empty) begin
            // Check if we have enough free FPUs
            if (free_tail - free_head >= fork_request_count) begin
                fork_grant <= 1'b1;
                
                // Allocate FPUs
                for (int i = 0; i < fork_request_count; i = i + 1) begin
                    new_process_ids[i] <= free_queue[free_head + i];
                    fpu_status[free_queue[free_head + i]] <= FPU_ALLOCATED;
                    
                    // Set parent-child relationship
                    process_parent[free_queue[free_head + i]] <= fork_request_from;
                end
                
                // Update free queue
                free_head <= free_head + fork_request_count;
                
                // Update child count for parent
                process_child_count[fork_request_from] <= 
                    process_child_count[fork_request_from] + fork_request_count;
            end
        end else begin
            fork_grant <= 1'b0;
        end
        
        // Handle termination
        if (terminate_valid) begin
            fpu_status[terminate_process_id] <= FPU_FREE;
            free_queue[free_tail] <= terminate_process_id;
            free_tail <= free_tail + 1;
            
            // Update parent's child count
            parent_id = process_parent[terminate_process_id];
            if (parent_id != INVALID_FPU_ID) begin
                process_child_count[parent_id] <= process_child_count[parent_id] - 1;
                
                // If all children terminated, notify parent
                if (process_child_count[parent_id] == 1) begin
                    // Send JOIN event to parent
                    if (!fpu_event_queue_full[parent_id]) begin
                        fpu_event_in[parent_id] <= EVENT_CHILDREN_TERMINATED;
                        fpu_event_valid[parent_id] <= 1'b1;
                    end
                end
            end
        end
    end
end
```

### 2.4 Memory Management Unit (MMU)

#### 2.4.1 Architecture

The MMU consists of:
- Memory allocation logic (manages memory blocks)
- Free list (tracks available memory blocks)
- Memory access arbitration (handles concurrent memory access)
- Garbage collection logic (reclaims memory from terminated processes)

#### 2.4.2 Memory Allocation

```verilog
// Memory allocation logic
always @(posedge clk or posedge reset) begin
    if (reset) begin
        // Initialize free list
        for (int i = 0; i < NUM_BLOCKS-1; i = i + 1) begin
            free_list[i] <= i + 1;
        end
        free_list[NUM_BLOCKS-1] <= NULL_BLOCK;
        free_head <= 0;
        alloc_grant <= 1'b0;
    end else begin
        // Handle allocation requests
        if (alloc_request_valid && free_head != NULL_BLOCK) begin
            // Allocate block
            alloc_address <= BASE_ADDRESS + (free_head * BLOCK_SIZE);
            alloc_grant <= 1'b1;
            
            // Update allocation table
            block_owner[free_head] <= alloc_request_from;
            
            // Update free list
            free_head <= free_list[free_head];
        end else begin
            alloc_grant <= 1'b0;
        end
        
        // Handle deallocation
        if (dealloc_valid) begin
            // Calculate block index
            block_index = (dealloc_address - BASE_ADDRESS) / BLOCK_SIZE;
            
            // Add to free list
            free_list[block_index] <= free_head;
            free_head <= block_index;
            
            // Clear owner
            block_owner[block_index] <= INVALID_FPU_ID;
        end
    end
end
```

### 2.5 Linux Interface Controller (LIC)

#### 2.5.1 Architecture

The LIC consists of:
- HPS interface (communicates with MiSTer's Linux OS)
- Command registers (control registers for core operation)
- Status registers (report core status to Linux)
- Program buffer (stores program data during loading)
- Debug interface (provides debugging information)

#### 2.5.2 HPS Interface

```verilog
// HPS interface logic
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
```

## 3. Integration with MiSTer Platform

### 3.1 MiSTer Core Structure

The MiSTer platform requires specific file organization and interfaces:

```
cores/rholang_fsm/
├── rtl/                    # Verilog source files
│   ├── rholang_fsm_core.v  # Top-level module
│   ├── fpu.v               # FSM Processing Unit
│   ├── ccn.v               # Channel Communication Network
│   ├── pcmu.v              # Process Creation and Management Unit
│   ├── mmu.v               # Memory Management Unit
│   ├── lic.v               # Linux Interface Controller
│   └── ...                 # Other modules
├── sys/                    # MiSTer system files
│   ├── sys_top.v           # MiSTer top-level wrapper
│   ├── pll.v               # PLL configuration
│   └── ...                 # Other system files
├── rholang_fsm.qpf         # Quartus project file
├── rholang_fsm.qsf         # Quartus settings file
├── rholang_fsm.sdc         # Timing constraints
└── rholang_fsm.sv          # SystemVerilog wrapper
```

### 3.2 MiSTer Interface Requirements

The core must implement the standard MiSTer interfaces:

1. **HPS Interface** - For communication with Linux OS
2. **SDRAM Interface** - For external memory access
3. **Video Interface** - For displaying status and debug information
4. **Input Interface** - For user control

### 3.3 Core Wrapper

```verilog
module rholang_fsm_wrapper (
    // MiSTer system interface
    input             clk_sys,
    input             reset_n,
    
    // HPS interface
    input      [31:0] h2f_user0_in,
    output     [31:0] h2f_user0_out,
    
    // SDRAM interface
    output     [12:0] SDRAM_A,
    output      [1:0] SDRAM_BA,
    inout      [15:0] SDRAM_DQ,
    output            SDRAM_DQML,
    output            SDRAM_DQMH,
    output            SDRAM_nCS,
    output            SDRAM_nCAS,
    output            SDRAM_nRAS,
    output            SDRAM_nWE,
    
    // Video interface
    output     [23:0] VIDEO_RGB,
    output            VIDEO_VS,
    output            VIDEO_HS,
    output            VIDEO_DE,
    
    // User inputs
    input      [31:0] joystick_0,
    input      [31:0] joystick_1,
    input      [15:0] joystick_analog_0,
    input      [15:0] joystick_analog_1,
    input      [31:0] status,
    input             ps2_key
);

    // Reset signal
    wire reset = ~reset_n;
    
    // Instantiate Rholang FSM Core
    Rholang_FSM_Core rholang_core (
        .clk(clk_sys),
        .reset(reset),
        .hps_writedata(h2f_user0_in),
        .hps_readdata(h2f_user0_out),
        .hps_address(h2f_user0_in[7:0]),
        .hps_write(h2f_user0_in[8]),
        .hps_read(h2f_user0_in[9]),
        .hps_waitrequest(),
        // SDRAM interface connections
        // ...
        // Debug interface
        .debug_leds()
    );
    
    // Video generation for status display
    // ...
    
    // Input handling
    // ...

endmodule
```

## 4. Testing and Validation Methodology

### 4.1 Simulation Testing

1. **Unit Tests**
   - Test each module individually with test benches
   - Verify state transitions and event handling
   - Test boundary conditions and error cases

2. **Integration Tests**
   - Test interaction between modules
   - Verify message passing between FPUs
   - Test process creation and termination

3. **System Tests**
   - Test complete core with simulated programs
   - Verify correct execution of Rholang constructs
   - Test performance and resource utilization

### 4.2 Hardware Testing

1. **FPGA Synthesis Tests**
   - Verify timing closure
   - Check resource utilization
   - Test on DE10-Nano board

2. **MiSTer Integration Tests**
   - Test Linux interface
   - Verify program loading
   - Test debugging interfaces

3. **Rholang Program Tests**
   - Test with simple Rholang programs
   - Verify correct execution results
   - Test concurrent behavior

### 4.3 Test Programs

A set of Rholang test programs will be created to validate the core:

1. **Hello World**
   ```rholang
   new stdout(`rho:io:stdout`) in {
     stdout!("Hello, Rholang on MiSTer FPGA!")
   }
   ```

2. **Parallel Processes**
   ```rholang
   new channel in {
     channel!("Message 1") |
     channel!("Message 2") |
     for (msg1 <- channel; msg2 <- channel) {
       new stdout(`rho:io:stdout`) in {
         stdout!(["Received", msg1, "and", msg2])
       }
     }
   }
   ```

3. **Fibonacci Calculator**
   ```rholang
   new fib, stdout(`rho:io:stdout`) in {
     contract fib(@n, ret) = {
       if (n == 0) { ret!(0) }
       else {
         if (n == 1) { ret!(1) }
         else {
           new a, b in {
             fib!(n - 1, *a) |
             fib!(n - 2, *b) |
             for (@x <- a; @y <- b) {
               ret!(x + y)
             }
           }
         }
       }
     } |
     new result in {
       fib!(10, *result) |
       for (@value <- result) {
         stdout!(["Fibonacci(10) =", value])
       }
     }
   }
   ```

## 5. Resource Utilization Estimates

### 5.1 DE10-Nano FPGA Resources

The Cyclone V FPGA on the DE10-Nano has:
- 110,000 Logic Elements (LEs)
- 5,570 Kb of embedded memory
- 112 variable-precision DSP blocks
- 3 PLLs

### 5.2 Estimated Resource Usage

| Component                        | Logic Elements | Memory (Kb) | DSP Blocks |
|----------------------------------|---------------|-------------|------------|
| FSM Processing Units (16 units)  | 32,000        | 1,600       | 32         |
| Channel Communication Network    | 15,000        | 800         | 0          |
| Process Creation & Management    | 8,000         | 400         | 0          |
| Memory Management Unit           | 10,000        | 1,200       | 0          |
| Linux Interface Controller       | 5,000         | 800         | 0          |
| MiSTer System Integration        | 10,000        | 400         | 8          |
| **Total**                        | **80,000**    | **5,200**   | **40**     |
| **Available**                    | **110,000**   | **5,570**   | **112**    |
| **Utilization**                  | **73%**       | **93%**     | **36%**    |

### 5.3 Performance Estimates

- **Clock Frequency**: 100 MHz
- **FSM Transitions per Second**: Up to 50 million
- **Concurrent Processes**: Up to 16
- **Channel Capacity**: 256 channels with up to 16 receivers each

## 6. Implementation Timeline

| Phase | Task | Duration |
|-------|------|----------|
| 1 | Development Environment Setup | 1 week |
| 2 | FSM Processing Unit Implementation | 3 weeks |
| 3 | Channel Communication Network Implementation | 2 weeks |
| 4 | Process & Memory Management Implementation | 2 weeks |
| 5 | Linux Interface Controller Implementation | 1 week |
| 6 | Integration and System Testing | 2 weeks |
| 7 | MiSTer Platform Integration | 1 week |
| 8 | Documentation and Sample Programs | 1 week |
| **Total** | | **13 weeks** |

## 7. Deliverables

### 7.1 Verilog HDL Codebase

Complete Verilog implementation of:
- FSM Processing Units
- Channel Communication Network
- Process Creation and Management Unit
- Memory Management Unit
- Linux Interface Controller
- Top-level integration

### 7.2 Quartus Project Files

- Project file (rholang_fsm.qpf)
- Settings file (rholang_fsm.qsf)
- Timing constraints (rholang_fsm.sdc)
- Pin assignments

### 7.3 MiSTer Integration Files

- Core binary (rholang_fsm.rbf)
- Launcher script (rholang_fsm.sh)
- Menu integration

### 7.4 Documentation

- Architecture description
- Implementation details
- User guide
- Testing results
- Performance analysis

### 7.5 Sample Rholang Programs

- Basic examples
- Concurrent processing examples
- Advanced examples demonstrating all features

## 8. Conclusion

This implementation plan provides a comprehensive approach to creating a Rholang FSM core for the MiSTer FPGA platform. By following this plan, we will create a hardware accelerator that accurately implements the Rholang execution model while integrating seamlessly with the MiSTer ecosystem.

The modular design allows for future enhancements and optimizations, while the comprehensive testing methodology ensures correct operation. The estimated resource utilization shows that the implementation is feasible on the DE10-Nano FPGA, with room for future expansion.