// Memory Management Unit (MMU) for Rholang FSM Core
// This module manages memory allocation and access for FSM instances

module Memory_Management_Unit (
    // Clock and reset
    input wire clk,
    input wire reset,
    
    // Memory allocation interface
    input wire [FPU_ID_WIDTH-1:0] alloc_request_from,
    input wire [15:0] alloc_size,
    input wire alloc_request_valid,
    output reg [31:0] alloc_address,
    output reg alloc_grant,
    
    // Memory deallocation interface
    input wire [31:0] dealloc_address,
    input wire [15:0] dealloc_size,
    input wire dealloc_valid,
    
    // Memory access interface - read port
    input wire [31:0] read_addr,
    input wire read_valid,
    output reg [31:0] read_data,
    output reg read_ready,
    
    // Memory access interface - write port
    input wire [31:0] write_addr,
    input wire [31:0] write_data,
    input wire write_valid,
    output reg write_ready,
    
    // SDRAM interface
    output reg [SDRAM_ADDR_WIDTH-1:0] sdram_addr,
    output reg sdram_write,
    output reg [SDRAM_DATA_WIDTH-1:0] sdram_write_data,
    input wire [SDRAM_DATA_WIDTH-1:0] sdram_read_data,
    output reg sdram_req,
    input wire sdram_ack,
    
    // Debug interface
    output reg [7:0] debug_status,
    output reg [15:0] debug_allocated_blocks,
    output reg [15:0] debug_free_blocks
);

    // Parameters
    parameter FPU_ID_WIDTH = 4;
    parameter NUM_FPUS = 16;
    parameter SDRAM_ADDR_WIDTH = 24;
    parameter SDRAM_DATA_WIDTH = 32;
    parameter NUM_BLOCKS = 1024;
    parameter BLOCK_SIZE = 256; // bytes
    parameter BASE_ADDRESS = 32'h10000000;
    parameter NULL_BLOCK = {16{1'b1}}; // Invalid block index
    
    // Memory block status
    localparam BLOCK_FREE = 2'b00;
    localparam BLOCK_ALLOCATED = 2'b01;
    localparam BLOCK_RESERVED = 2'b10;
    
    // Memory allocation table
    reg [1:0] block_status [NUM_BLOCKS-1:0]; // 0=free, 1=allocated, 2=reserved
    reg [FPU_ID_WIDTH-1:0] block_owner [NUM_BLOCKS-1:0]; // Owner FPU ID
    reg [15:0] block_size [NUM_BLOCKS-1:0]; // Size of allocation in bytes
    
    // Free block list - implemented as a linked list
    reg [15:0] free_list [NUM_BLOCKS-1:0]; // Next free block index
    reg [15:0] free_head; // Head of free list
    
    // Memory access state machine
    localparam MEM_IDLE = 2'b00;
    localparam MEM_READ = 2'b01;
    localparam MEM_WRITE = 2'b10;
    localparam MEM_WAIT = 2'b11;
    
    reg [1:0] mem_state;
    reg [31:0] mem_addr;
    reg [31:0] mem_data;
    
    // Debug counters
    reg [15:0] allocated_blocks;
    reg [15:0] free_blocks;
    
    // Initialize memory allocation table and free list
    integer i;
    initial begin
        for (i = 0; i < NUM_BLOCKS; i = i + 1) begin
            block_status[i] = BLOCK_FREE;
            block_owner[i] = {FPU_ID_WIDTH{1'b1}}; // Invalid FPU ID
            block_size[i] = 0;
            
            // Initialize free list as a linked list
            if (i < NUM_BLOCKS-1)
                free_list[i] = i + 1;
            else
                free_list[i] = NULL_BLOCK;
        end
        free_head = 0;
        allocated_blocks = 0;
        free_blocks = NUM_BLOCKS;
        
        // Initialize memory access state machine
        mem_state = MEM_IDLE;
    end
    
    // Memory allocation logic
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            // Reset allocation table
            for (i = 0; i < NUM_BLOCKS; i = i + 1) begin
                block_status[i] <= BLOCK_FREE;
                block_owner[i] <= {FPU_ID_WIDTH{1'b1}}; // Invalid FPU ID
                block_size[i] <= 0;
                
                // Reset free list
                if (i < NUM_BLOCKS-1)
                    free_list[i] <= i + 1;
                else
                    free_list[i] <= NULL_BLOCK;
            end
            free_head <= 0;
            allocated_blocks <= 0;
            free_blocks <= NUM_BLOCKS;
            
            // Reset control signals
            alloc_grant <= 1'b0;
            alloc_address <= 32'h0;
        end else begin
            // Default values
            alloc_grant <= 1'b0;
            
            // Handle allocation requests
            if (alloc_request_valid && free_head != NULL_BLOCK) begin
                // Calculate number of blocks needed
                reg [15:0] blocks_needed;
                blocks_needed = (alloc_size + BLOCK_SIZE - 1) / BLOCK_SIZE; // Ceiling division
                
                // Check if we have enough contiguous blocks
                // This is a simplified allocation strategy - a real implementation
                // would use a more sophisticated algorithm
                if (free_blocks >= blocks_needed) begin
                    // Allocate first block
                    reg [15:0] block_index = free_head;
                    
                    // Update free list head
                    free_head <= free_list[free_head];
                    
                    // Mark block as allocated
                    block_status[block_index] <= BLOCK_ALLOCATED;
                    block_owner[block_index] <= alloc_request_from;
                    block_size[block_index] <= alloc_size;
                    
                    // Calculate and return address
                    alloc_address <= BASE_ADDRESS + (block_index * BLOCK_SIZE);
                    alloc_grant <= 1'b1;
                    
                    // Update counters
                    allocated_blocks <= allocated_blocks + 1;
                    free_blocks <= free_blocks - 1;
                end
            end
            
            // Handle deallocation requests
            if (dealloc_valid) begin
                // Calculate block index from address
                reg [15:0] block_index;
                block_index = (dealloc_address - BASE_ADDRESS) / BLOCK_SIZE;
                
                // Check if block is allocated
                if (block_status[block_index] == BLOCK_ALLOCATED) begin
                    // Mark as free
                    block_status[block_index] <= BLOCK_FREE;
                    block_owner[block_index] <= {FPU_ID_WIDTH{1'b1}}; // Invalid FPU ID
                    
                    // Add to free list
                    free_list[block_index] <= free_head;
                    free_head <= block_index;
                    
                    // Update counters
                    allocated_blocks <= allocated_blocks - 1;
                    free_blocks <= free_blocks + 1;
                end
            end
        end
    end
    
    // Memory access state machine
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            mem_state <= MEM_IDLE;
            read_ready <= 1'b0;
            write_ready <= 1'b0;
            sdram_req <= 1'b0;
            sdram_write <= 1'b0;
        end else begin
            // Default values
            read_ready <= 1'b0;
            write_ready <= 1'b0;
            
            case (mem_state)
                MEM_IDLE: begin
                    // Handle read requests with priority
                    if (read_valid) begin
                        mem_state <= MEM_READ;
                        mem_addr <= read_addr;
                        sdram_addr <= read_addr[SDRAM_ADDR_WIDTH-1:0];
                        sdram_req <= 1'b1;
                        sdram_write <= 1'b0;
                    end
                    // Handle write requests
                    else if (write_valid) begin
                        mem_state <= MEM_WRITE;
                        mem_addr <= write_addr;
                        mem_data <= write_data;
                        sdram_addr <= write_addr[SDRAM_ADDR_WIDTH-1:0];
                        sdram_write_data <= write_data;
                        sdram_req <= 1'b1;
                        sdram_write <= 1'b1;
                    end
                end
                
                MEM_READ: begin
                    if (sdram_ack) begin
                        // Read complete
                        read_data <= sdram_read_data;
                        read_ready <= 1'b1;
                        sdram_req <= 1'b0;
                        mem_state <= MEM_IDLE;
                    end
                end
                
                MEM_WRITE: begin
                    if (sdram_ack) begin
                        // Write complete
                        write_ready <= 1'b1;
                        sdram_req <= 1'b0;
                        sdram_write <= 1'b0;
                        mem_state <= MEM_IDLE;
                    end
                end
                
                default: begin
                    // Should never get here
                    mem_state <= MEM_IDLE;
                end
            endcase
        end
    end
    
    // Garbage collection
    // In a real implementation, this would periodically scan for orphaned blocks
    // and reclaim them. This is a simplified version that doesn't actually
    // perform garbage collection.
    
    // Debug output
    always @(posedge clk) begin
        debug_allocated_blocks <= allocated_blocks;
        debug_free_blocks <= free_blocks;
        debug_status <= {mem_state, 2'b00, 1'b0, 1'b0, 1'b0, 1'b0}; // Status bits
    end

endmodule