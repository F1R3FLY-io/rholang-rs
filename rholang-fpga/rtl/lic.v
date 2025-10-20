// Linux Interface Controller (LIC) for Rholang FSM Core
// This module interfaces with the MiSTer Linux OS

module Linux_Interface_Controller (
    // Clock and reset
    input wire clk,
    input wire reset,
    
    // HPS interface
    input wire [31:0] hps_writedata,
    output reg [31:0] hps_readdata,
    input wire [7:0] hps_address,
    input wire hps_write,
    input wire hps_read,
    output reg hps_waitrequest,
    
    // FSM control interface
    output reg [31:0] program_data,
    output reg program_valid,
    input wire program_ready,
    output reg start_execution,
    input wire execution_done,
    
    // Status and debug interface
    input wire [31:0] status_data [NUM_STATUS_REGS-1:0],
    input wire [7:0] status_address,
    input wire status_read,
    output reg status_valid,
    
    // Debug interface
    output reg [7:0] debug_status,
    output reg [15:0] debug_program_counter,
    output reg [7:0] debug_state
);

    // Parameters
    parameter NUM_STATUS_REGS = 16;
    parameter PROGRAM_BUFFER_SIZE = 1024;
    
    // Register addresses
    localparam REG_CONTROL        = 8'h00;
    localparam REG_STATUS         = 8'h04;
    localparam REG_PROGRAM_DATA   = 8'h08;
    localparam REG_PROGRAM_COUNT  = 8'h0C;
    localparam REG_DEBUG_CONTROL  = 8'h10;
    localparam REG_DEBUG_DATA     = 8'h14;
    
    // Control register bits
    localparam CTRL_START_EXEC    = 0;
    localparam CTRL_LOAD_PROGRAM  = 1;
    localparam CTRL_RESET_CORE    = 2;
    localparam CTRL_DEBUG_ENABLE  = 3;
    
    // Status register bits
    localparam STAT_EXEC_DONE     = 0;
    localparam STAT_PROGRAM_READY = 1;
    localparam STAT_CORE_BUSY     = 2;
    localparam STAT_ERROR         = 3;
    
    // Internal registers
    reg [31:0] control_reg;
    reg [31:0] status_reg;
    reg [31:0] program_counter;
    reg [31:0] debug_control_reg;
    reg [31:0] debug_data_reg;
    
    // Program buffer
    reg [31:0] program_buffer [PROGRAM_BUFFER_SIZE-1:0];
    
    // State machine for program loading
    localparam LOAD_IDLE = 2'b00;
    localparam LOAD_STREAMING = 2'b01;
    localparam LOAD_WAIT = 2'b10;
    
    reg [1:0] load_state;
    reg [31:0] stream_counter;
    
    // Debug state
    reg [7:0] lic_state;
    
    // Initialize registers
    initial begin
        control_reg = 32'h0;
        status_reg = 32'h0;
        program_counter = 32'h0;
        debug_control_reg = 32'h0;
        debug_data_reg = 32'h0;
        load_state = LOAD_IDLE;
        stream_counter = 32'h0;
        lic_state = 8'h0;
    end
    
    // HPS interface logic
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            control_reg <= 32'h0;
            status_reg <= 32'h0;
            program_counter <= 32'h0;
            debug_control_reg <= 32'h0;
            debug_data_reg <= 32'h0;
            hps_waitrequest <= 1'b0;
            hps_readdata <= 32'h0;
            load_state <= LOAD_IDLE;
            stream_counter <= 32'h0;
            lic_state <= 8'h0;
        end else begin
            // Default values
            hps_waitrequest <= 1'b0;
            
            // Handle HPS writes
            if (hps_write) begin
                case (hps_address)
                    REG_CONTROL: begin
                        control_reg <= hps_writedata;
                        
                        // Check for program load request
                        if (hps_writedata[CTRL_LOAD_PROGRAM] && !control_reg[CTRL_LOAD_PROGRAM]) begin
                            // Start program loading
                            load_state <= LOAD_IDLE;
                            program_counter <= 32'h0;
                        end
                    end
                    
                    REG_PROGRAM_DATA: begin
                        // Store program data in buffer
                        if (program_counter < PROGRAM_BUFFER_SIZE) begin
                            program_buffer[program_counter] <= hps_writedata;
                            program_counter <= program_counter + 1;
                        end
                    end
                    
                    REG_DEBUG_CONTROL: begin
                        debug_control_reg <= hps_writedata;
                    end
                    
                    default: begin
                        // Ignore writes to other addresses
                    end
                endcase
            end
            
            // Handle HPS reads
            if (hps_read) begin
                case (hps_address)
                    REG_CONTROL: begin
                        hps_readdata <= control_reg;
                    end
                    
                    REG_STATUS: begin
                        hps_readdata <= status_reg;
                    end
                    
                    REG_PROGRAM_COUNT: begin
                        hps_readdata <= program_counter;
                    end
                    
                    REG_DEBUG_CONTROL: begin
                        hps_readdata <= debug_control_reg;
                    end
                    
                    REG_DEBUG_DATA: begin
                        hps_readdata <= debug_data_reg;
                    end
                    
                    default: begin
                        // For other addresses, check if it's a status register
                        if (hps_address >= 8'h20 && hps_address < 8'h20 + (NUM_STATUS_REGS * 4)) begin
                            // Calculate status register index
                            reg [7:0] status_idx = (hps_address - 8'h20) >> 2;
                            hps_readdata <= status_data[status_idx];
                        end else begin
                            hps_readdata <= 32'h0;
                        end
                    end
                endcase
            end
            
            // Update status register
            status_reg[STAT_EXEC_DONE] <= execution_done;
            status_reg[STAT_PROGRAM_READY] <= (program_counter > 0);
            status_reg[STAT_CORE_BUSY] <= (load_state != LOAD_IDLE) || start_execution;
            
            // Handle control register bits
            start_execution <= control_reg[CTRL_START_EXEC];
            
            // Program loading state machine
            case (load_state)
                LOAD_IDLE: begin
                    program_valid <= 1'b0;
                    
                    // Check if program loading is requested
                    if (control_reg[CTRL_LOAD_PROGRAM] && program_counter > 0) begin
                        load_state <= LOAD_STREAMING;
                        stream_counter <= 0;
                    end
                end
                
                LOAD_STREAMING: begin
                    // Stream program data to core
                    if (stream_counter < program_counter) begin
                        program_data <= program_buffer[stream_counter];
                        program_valid <= 1'b1;
                        
                        if (program_ready) begin
                            // Core accepted data, move to next word
                            stream_counter <= stream_counter + 1;
                        end
                    end else begin
                        // All data streamed
                        program_valid <= 1'b0;
                        load_state <= LOAD_IDLE;
                        control_reg[CTRL_LOAD_PROGRAM] <= 1'b0; // Auto-clear load bit
                    end
                end
                
                default: begin
                    load_state <= LOAD_IDLE;
                end
            endcase
            
            // Update debug state
            lic_state <= {load_state, 2'b00, control_reg[CTRL_START_EXEC], 
                          control_reg[CTRL_LOAD_PROGRAM], execution_done, 1'b0};
        end
    end
    
    // Status interface logic
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            status_valid <= 1'b0;
        end else begin
            // Handle status read requests
            if (status_read) begin
                // In a real implementation, this would fetch status data
                // from various components of the core
                status_valid <= 1'b1;
            end else begin
                status_valid <= 1'b0;
            end
        end
    end
    
    // Debug output
    always @(posedge clk) begin
        debug_status <= lic_state;
        debug_program_counter <= program_counter[15:0];
        debug_state <= {load_state, 2'b00, 4'h0};
    end

endmodule