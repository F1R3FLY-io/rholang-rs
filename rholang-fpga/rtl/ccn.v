// Channel Communication Network (CCN) for Rholang FSM Core
// This module implements message passing between FSM instances

module Channel_Communication_Network (
    // Clock and reset
    input wire clk,
    input wire reset,
    
    // FPU interfaces (parameterized for multiple FPUs)
    input wire [7:0] fpu_channel_id [NUM_FPUS-1:0],
    input wire [31:0] fpu_message [NUM_FPUS-1:0],
    input wire fpu_send_valid [NUM_FPUS-1:0],
    output reg fpu_send_ready [NUM_FPUS-1:0],
    
    output reg [7:0] fpu_recv_channel_id [NUM_FPUS-1:0],
    output reg [31:0] fpu_recv_message [NUM_FPUS-1:0],
    output reg fpu_recv_valid [NUM_FPUS-1:0],
    input wire fpu_recv_ready [NUM_FPUS-1:0],
    
    // Channel registration interface
    input wire [7:0] register_channel_id [NUM_FPUS-1:0],
    input wire register_channel_valid [NUM_FPUS-1:0],
    output reg register_channel_ready [NUM_FPUS-1:0],
    
    input wire [7:0] unregister_channel_id [NUM_FPUS-1:0],
    input wire unregister_channel_valid [NUM_FPUS-1:0],
    output reg unregister_channel_ready [NUM_FPUS-1:0],
    
    // Debug interface
    output reg [7:0] debug_status,
    output reg [15:0] debug_channel_count,
    output reg [15:0] debug_message_count
);

    // Parameters
    parameter NUM_FPUS = 16;
    parameter NUM_CHANNELS = 256;
    parameter MAX_RECEIVERS = 16;
    parameter QUEUE_DEPTH = 16;
    
    // Channel table - maps channel IDs to receiving FPUs
    reg [3:0] channel_table [NUM_CHANNELS-1:0][MAX_RECEIVERS-1:0];
    reg [3:0] receiver_count [NUM_CHANNELS-1:0]; // Count of receivers for each channel
    
    // Message queues for each channel
    reg [31:0] message_queue [NUM_CHANNELS-1:0][QUEUE_DEPTH-1:0];
    reg [3:0] queue_head [NUM_CHANNELS-1:0];
    reg [3:0] queue_tail [NUM_CHANNELS-1:0];
    
    // Helper wires for queue status
    wire queue_empty [NUM_CHANNELS-1:0];
    wire queue_full [NUM_CHANNELS-1:0];
    
    // Generate queue status signals
    genvar ch;
    generate
        for (ch = 0; ch < NUM_CHANNELS; ch = ch + 1) begin: queue_status
            assign queue_empty[ch] = (queue_head[ch] == queue_tail[ch]);
            assign queue_full[ch] = ((queue_tail[ch] + 1) & 4'hF) == queue_head[ch];
        end
    endgenerate
    
    // Debug counters
    reg [15:0] total_channels;
    reg [15:0] total_messages;
    
    // Initialize channel table and queues
    integer i, j;
    initial begin
        for (i = 0; i < NUM_CHANNELS; i = i + 1) begin
            receiver_count[i] = 0;
            queue_head[i] = 0;
            queue_tail[i] = 0;
            for (j = 0; j < MAX_RECEIVERS; j = j + 1) begin
                channel_table[i][j] = 4'hF; // Invalid FPU ID
            end
        end
        total_channels = 0;
        total_messages = 0;
    end
    
    // Channel registration logic
    integer fpu_idx, channel_idx, recv_idx;
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            // Reset channel table
            for (channel_idx = 0; channel_idx < NUM_CHANNELS; channel_idx = channel_idx + 1) begin
                receiver_count[channel_idx] <= 0;
                for (recv_idx = 0; recv_idx < MAX_RECEIVERS; recv_idx = recv_idx + 1) begin
                    channel_table[channel_idx][recv_idx] <= 4'hF; // Invalid FPU ID
                end
            end
            total_channels <= 0;
            
            // Reset registration ready signals
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                register_channel_ready[fpu_idx] <= 1'b1;
                unregister_channel_ready[fpu_idx] <= 1'b1;
            end
        end else begin
            // Default values for ready signals
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                register_channel_ready[fpu_idx] <= 1'b1;
                unregister_channel_ready[fpu_idx] <= 1'b1;
            end
            
            // Handle channel registrations
            for (fpu_idx = 0; fpu_idx < NUM_FPUS; fpu_idx = fpu_idx + 1) begin
                if (register_channel_valid[fpu_idx]) begin
                    channel_idx = register_channel_id[fpu_idx];
                    
                    // Check if there's room for another receiver
                    if (receiver_count[channel_idx] < MAX_RECEIVERS) begin
                        // Add FPU to channel's receiver list
                        channel_table[channel_idx][receiver_count[channel_idx]] <= fpu_idx[3:0];
                        receiver_count[channel_idx] <= receiver_count[channel_idx] + 1;
                        
                        // Update total channel count if this is the first receiver
                        if (receiver_count[channel_idx] == 0) begin
                            total_channels <= total_channels + 1;
                        end
                    end else begin
                        // No room for more receivers
                        register_channel_ready[fpu_idx] <= 1'b0;
                    end
                end
                
                // Handle channel unregistrations
                if (unregister_channel_valid[fpu_idx]) begin
                    channel_idx = unregister_channel_id[fpu_idx];
                    
                    // Find FPU in channel's receiver list
                    for (recv_idx = 0; recv_idx < MAX_RECEIVERS; recv_idx = recv_idx + 1) begin
                        if (channel_table[channel_idx][recv_idx] == fpu_idx[3:0]) begin
                            // Remove FPU by replacing with last receiver
                            channel_table[channel_idx][recv_idx] <= 
                                channel_table[channel_idx][receiver_count[channel_idx] - 1];
                            channel_table[channel_idx][receiver_count[channel_idx] - 1] <= 4'hF;
                            receiver_count[channel_idx] <= receiver_count[channel_idx] - 1;
                            
                            // Update total channel count if this was the last receiver
                            if (receiver_count[channel_idx] == 1) begin
                                total_channels <= total_channels - 1;
                            end
                            
                            // Exit loop after finding and removing
                            recv_idx = MAX_RECEIVERS;
                        end
                    end
                end
            end
        end
    end
    
    // Message sending logic
    integer sender_idx;
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            // Reset message queues
            for (channel_idx = 0; channel_idx < NUM_CHANNELS; channel_idx = channel_idx + 1) begin
                queue_head[channel_idx] <= 0;
                queue_tail[channel_idx] <= 0;
            end
            total_messages <= 0;
            
            // Reset send ready signals
            for (sender_idx = 0; sender_idx < NUM_FPUS; sender_idx = sender_idx + 1) begin
                fpu_send_ready[sender_idx] <= 1'b1;
            end
        end else begin
            // Default values for send ready signals
            for (sender_idx = 0; sender_idx < NUM_FPUS; sender_idx = sender_idx + 1) begin
                fpu_send_ready[sender_idx] <= 1'b1;
            end
            
            // Handle message sending
            for (sender_idx = 0; sender_idx < NUM_FPUS; sender_idx = sender_idx + 1) begin
                if (fpu_send_valid[sender_idx]) begin
                    channel_idx = fpu_channel_id[sender_idx];
                    
                    // Check if queue has space
                    if (!queue_full[channel_idx]) begin
                        // Add message to queue
                        message_queue[channel_idx][queue_tail[channel_idx]] <= fpu_message[sender_idx];
                        queue_tail[channel_idx] <= (queue_tail[channel_idx] + 1) & 4'hF;
                        total_messages <= total_messages + 1;
                    end else begin
                        // Queue full, can't accept message
                        fpu_send_ready[sender_idx] <= 1'b0;
                    end
                end
            end
        end
    end
    
    // Message receiving logic
    integer recv_fpu_idx;
    always @(posedge clk or posedge reset) begin
        if (reset) begin
            // Reset receive signals
            for (recv_fpu_idx = 0; recv_fpu_idx < NUM_FPUS; recv_fpu_idx = recv_fpu_idx + 1) begin
                fpu_recv_valid[recv_fpu_idx] <= 1'b0;
                fpu_recv_channel_id[recv_fpu_idx] <= 8'h0;
                fpu_recv_message[recv_fpu_idx] <= 32'h0;
            end
        end else begin
            // Default values for receive signals
            for (recv_fpu_idx = 0; recv_fpu_idx < NUM_FPUS; recv_fpu_idx = recv_fpu_idx + 1) begin
                fpu_recv_valid[recv_fpu_idx] <= 1'b0;
            end
            
            // Process each channel
            for (channel_idx = 0; channel_idx < NUM_CHANNELS; channel_idx = channel_idx + 1) begin
                if (!queue_empty[channel_idx] && receiver_count[channel_idx] > 0) begin
                    // Get message from queue
                    reg [31:0] message = message_queue[channel_idx][queue_head[channel_idx]];
                    
                    // Flag to track if message was delivered to at least one receiver
                    reg message_delivered = 1'b0;
                    
                    // Try to deliver to each registered receiver
                    for (recv_idx = 0; recv_idx < receiver_count[channel_idx]; recv_idx = recv_idx + 1) begin
                        recv_fpu_idx = channel_table[channel_idx][recv_idx];
                        
                        // Check if receiver is ready
                        if (fpu_recv_ready[recv_fpu_idx] && !fpu_recv_valid[recv_fpu_idx]) begin
                            // Deliver message
                            fpu_recv_valid[recv_fpu_idx] <= 1'b1;
                            fpu_recv_channel_id[recv_fpu_idx] <= channel_idx[7:0];
                            fpu_recv_message[recv_fpu_idx] <= message;
                            message_delivered = 1'b1;
                            
                            // For persistent receives, deliver to all receivers
                            // For non-persistent, deliver to first ready receiver only
                            // This is simplified - actual persistence would be handled by FPU
                            // break;
                        end
                    end
                    
                    // If message was delivered, remove from queue
                    if (message_delivered) begin
                        queue_head[channel_idx] <= (queue_head[channel_idx] + 1) & 4'hF;
                        total_messages <= total_messages - 1;
                    end
                end
            end
        end
    end
    
    // Debug output
    always @(posedge clk) begin
        debug_channel_count <= total_channels;
        debug_message_count <= total_messages;
        debug_status <= {4'h0, 1'b0, 1'b0, 1'b0, 1'b0}; // Status bits
    end

endmodule