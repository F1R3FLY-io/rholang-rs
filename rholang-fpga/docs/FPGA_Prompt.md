# Prompt for MiSTer FPGA Rholang FSM Core Implementation

You are tasked with implementing a Rholang Finite State Machine (FSM) as an FPGA core for the MiSTer platform. This requires expertise in hardware design, concurrent programming models, and the MiSTer FPGA ecosystem.

## Project Requirements

Create a complete hardware implementation of the Rholang FSM execution model as described in the language specification. The implementation should:

1. Accurately model the Rholang FSM states and transitions
2. Implement the concurrent message-passing semantics of Rholang
3. Support the full range of Rholang operations including pattern matching and process logic
4. Integrate with the MiSTer FPGA Linux-based ecosystem
5. Provide appropriate debugging and monitoring interfaces

## Implementation Details

Please provide a comprehensive implementation plan including:

1. System architecture with detailed block diagrams
2. Verilog HDL code for key components:
   - FSM Processing Units
   - Channel Communication Network
   - Process Creation and Management Unit
   - Memory Management Unit
   - Linux Interface Controller
3. Integration strategy with MiSTer platform
4. Testing and validation methodology
5. Resource utilization estimates for the DE10-Nano FPGA

## Technical Constraints

- Target the Cyclone V FPGA on the DE10-Nano board
- Work within MiSTer's Linux-based core architecture
- Optimize for efficient use of FPGA resources
- Ensure compatibility with existing MiSTer cores and infrastructure

## Rholang FSM Model Reference

The Rholang FSM model includes the following key components:

### States
- INITIAL, EVALUATING, SENDING, RECEIVING, WAITING, BRANCHING, FORKING, JOINING, BINDING, MATCHING, CONSTRUCTING, TERMINATED

### Transitions
- EVALUATE, SEND, RECEIVE, FORK, JOIN, BIND, MATCH, BRANCH, CONSTRUCT, TERMINATE

### Events
- MESSAGE_AVAILABLE, CONDITION_MET, EXPRESSION_EVALUATED, PATTERN_MATCHED, TIMEOUT, ERROR

## Deliverables

1. Complete Verilog HDL codebase for the Rholang FSM core
2. Quartus project files for synthesis and implementation
3. MiSTer integration files (rbf, scripts, etc.)
4. Comprehensive documentation including:
   - Architecture description
   - Implementation details
   - User guide
   - Testing results
5. Sample Rholang programs demonstrating core functionality

Provide a detailed implementation plan that addresses all aspects of this project, with particular focus on how the Rholang FSM model will be realized in hardware.
