// Process Creation and Management Unit (PCMU) for Rholang FSM Core
// This module manages the lifecycle of FSM instances

module Process_Creation_Management_Unit (
    // Clock and reset
    input wire clk,
    input wire reset,
    
    // FPU control interfaces
    input wire [FPU_ID_WIDTH-1:0] fork_request_from [NUM_FPUS-1:0],
    input wire [3:0] fork_request_count [NUM_FPUS-1:0],
    input wire fork_request_valid [NUM_FPUS-1:0],
    output reg fork_grant [NUM_FPUS-1:0],
    output reg [FPU_ID_WIDTH-1:0] new_process_ids [NUM_FPUS-1:0][MAX_FORK-1:0],
    
    // Process termination interface
    input wire [FPU_ID_WIDTH-1:0] terminate_process_id [NUM_FPUS-1:0],
    input wire terminate_valid [NUM_FPUS-1:0],
    
    // FPU initialization interface
    output reg [FPU_ID_WIDTH-1:0] init_fpu_id,
    output reg [3:0] init_process_type,
    output reg [31:0] init_process_data,
    output reg init_valid,
    input wire init_ready,
    
    // Process scheduling interface
    output reg [FPU_ID_WIDTH-1:0] schedule_fpu_id,
    output reg schedule_valid,
    input wire schedule_ready,
    
    // Debug interface
    output reg [7:0] debug_status,
    output reg [15:0] debug_active_processes,
    output reg [15:0] debug_free_fpus
);

    // Parameters
    parameter NUM_FPUS = 16;
    parameter FPU_ID_WIDTH = 4;
    parameter MAX_FORK = 4;
    parameter INVALID_FPU_ID = {FPU_ID_WIDTH{1'b1}};
    
    // FPU status constants
    localparam FPU_FREE = 2'b00;
    localparam FPU_ALLOCATED = 2'b01;
    localparam FPU_RUNNING = 2'b10;
    localparam FPU_TERMINATING = 2'b11;
    
    // Process allocation table
    reg [1:0] fpu_status [NUM_FPUS-1:0]; // 0=free, 1=allocated, 2=running, 3=terminating
    reg [FPU_ID_WIDTH-1:0] process_parent [NUM_FPUS-1:0]; // Parent process ID
    reg [3:0] process_child_count [NUM_FPUS-1:0]; // Number of child processes
    reg [3:0] process_type [NUM_FPUS-1:0]; // Type of process
    
    // Free FPU queue
    reg [FPU_ID_WIDTH-1:0] free_queue [NUM_FPUS-1:0];
    reg [FPU_ID_WIDTH-1:0] free_head, free_tail;
    wire free_queue_empty = (free_head == free_tail);
    wire [FPU_ID_WIDTH-1:0] free_count = (free_tail >= free_head) ? 
                                         (free_tail - free_head) : 
                                         (NUM_FPUS - free_head + free_tail);
    
    // Debug counters
    reg [15:0] active_processes;
    
    // Initialize FPU status and free queue
    integer i;
    initial begin
        for (i = 0; i < NUM_FPUS; i = i + 1) begin
            fpu_status[i] = FPU_FREE;
            free_queue[i] = i[FPU_ID_WIDTH-1:0];
            process_parent[i] = INVALID_FPU_ID;
            process_child_count[i] = 0;
            process_type[i] = 0;
        end
        free_head = 0;
        free_tail = NUM_FPUS[FPU_ID_WIDTH-1:0];
        active_processes = 0;
    end
    
    // Fork request handling
    integer fpu_idx, child_idx;
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            // Initialize all FPUs as free
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                fpu_status[fpu_idx] <= FPU_FREE;
                free_queue[fpu_idx] <= fpu_idx[FPU_ID_WIDTH-1:0];
                process_parent[fpu_idx] <= INVALID_FPU_ID;
                process_child_count[fpu_idx] <= 0;
                process_type[fpu_idx] <= 0;
            end
            free_head <= 0;
            free_tail <= NUM_FPUS[FPU_ID_WIDTH-1:0];
            active_processes <= 0;
            
            // Reset control signals
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                fork_grant[fpu_idx] <= 1'b0;
                for (child_idx = 0; child_idx < MAX_FORK; child_idx = child_idx + 1) begin
                    new_process_ids[fpu_idx][child_idx] <= INVALID_FPU_ID;
                end
            end
            init_valid <= 1'b0;
            schedule_valid <= 1'b0;
        end else begin
            // Default values for control signals
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                fork_grant[fpu_idx] <= 1'b0;
            end
            init_valid <= 1'b0;
            schedule_valid <= 1'b0;
            
            // Handle fork requests
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                if (fork_request_valid[fpu_idx] && !free_queue_empty) begin
                    // Check if we have enough free FPUs
                    if (free_count >= fork_request_count[fpu_idx]) begin
                        fork_grant[fpu_idx] <= 1'b1;
                        
                        // Allocate FPUs for child processes
                        for (child_idx = 0; child_idx < fork_request_count[fpu_idx]; child_idx = child_idx + 1) begin
                            // Get next free FPU
                            new_process_ids[fpu_idx][child_idx] <= free_queue[free_head];
                            
                            // Mark as allocated
                            fpu_status[free_queue[free_head]] <= FPU_ALLOCATED;
                            
                            // Set parent-child relationship
                            process_parent[free_queue[free_head]] <= fork_request_from[fpu_idx];
                            
                            // Update free queue
                            free_head <= (free_head + 1) % NUM_FPUS;
                        end
                        
                        // Update child count for parent
                        process_child_count[fork_request_from[fpu_idx]] <= 
                            process_child_count[fork_request_from[fpu_idx]] + fork_request_count[fpu_idx];
                            
                        // Update active process count
                        active_processes <= active_processes + fork_request_count[fpu_idx];
                    end
                end
            end
            
            // Handle process termination
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                if (terminate_valid[fpu_idx]) begin
                    // Get process ID to terminate
                    reg [FPU_ID_WIDTH-1:0] pid = terminate_process_id[fpu_idx];
                    
                    // Mark as free
                    fpu_status[pid] <= FPU_FREE;
                    
                    // Add to free queue
                    free_queue[free_tail] <= pid;
                    free_tail <= (free_tail + 1) % NUM_FPUS;
                    
                    // Update active process count
                    active_processes <= active_processes - 1;
                    
                    // Update parent's child count
                    if (process_parent[pid] != INVALID_FPU_ID) begin
                        process_child_count[process_parent[pid]] <= 
                            process_child_count[process_parent[pid]] - 1;
                            
                        // If all children terminated, notify parent
                        if (process_child_count[process_parent[pid]] == 1) begin
                            // This would be handled by sending an event to the parent FPU
                            // In the actual implementation, this would be done through the event system
                        end
                    end
                    
                    // Clear parent reference
                    process_parent[pid] <= INVALID_FPU_ID;
                end
            end
            
            // Process initialization
            // This would initialize newly allocated FPUs with their process data
            // In a real implementation, this would be more complex and handle
            // different process types and initialization data
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                if (fpu_status[fpu_idx] == FPU_ALLOCATED && !init_valid && init_ready) begin
                    init_fpu_id <= fpu_idx[FPU_ID_WIDTH-1:0];
                    init_process_type <= process_type[fpu_idx];
                    init_process_data <= 32'h0; // Would be actual process data
                    init_valid <= 1'b1;
                    
                    // Mark as running
                    fpu_status[fpu_idx] <= FPU_RUNNING;
                end
            end
            
            // Process scheduling
            // This would select the next FPU to run
            // In a real implementation, this would implement a scheduling algorithm
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                if (fpu_status[fpu_idx] == FPU_RUNNING && !schedule_valid && schedule_ready) begin
                    schedule_fpu_id <= fpu_idx[FPU_ID_WIDTH-1:0];
                    schedule_valid <= 1'b1;
                    break; // Schedule one FPU at a time
                end
            end
        end
    end
    
    // Debug output
    always @(posedge clk) begin
        debug_active_processes <= active_processes;
        debug_free_fpus <= free_count;
        debug_status <= {4'h0, 1'b0, 1'b0, 1'b0, 1'b0}; // Status bits
    end

endmodule