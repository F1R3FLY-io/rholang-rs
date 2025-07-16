#**************************************************************
# Rholang FSM Core - Timing Constraints
#**************************************************************

#**************************************************************
# Create Clock
#**************************************************************
create_clock -name FPGA_CLK1_50 -period 20.000 [get_ports {FPGA_CLK1_50}]
create_clock -name FPGA_CLK2_50 -period 20.000 [get_ports {FPGA_CLK2_50}]
create_clock -name FPGA_CLK3_50 -period 20.000 [get_ports {FPGA_CLK3_50}]

# SDRAM clock
create_generated_clock -name SDRAM_CLK -source [get_pins {pll|pll_inst|altera_pll_i|outclk_wire[0]~CLKENA0|outclk}] [get_ports {SDRAM_CLK}]

# HDMI clocks
create_generated_clock -name HDMI_CLK -source [get_pins {pll|pll_inst|altera_pll_i|outclk_wire[1]~CLKENA0|outclk}] [get_ports {HDMI_TX_CLK}]

#**************************************************************
# Create Generated Clock
#**************************************************************
derive_pll_clocks

#**************************************************************
# Set Clock Uncertainty
#**************************************************************
derive_clock_uncertainty

#**************************************************************
# Set Input Delay
#**************************************************************
# SDRAM input delays
set_input_delay -clock SDRAM_CLK -max 6.4 [get_ports {SDRAM_DQ[*]}]
set_input_delay -clock SDRAM_CLK -min 3.2 [get_ports {SDRAM_DQ[*]}]

# HPS input delays
# These are handled by the HPS IP

#**************************************************************
# Set Output Delay
#**************************************************************
# SDRAM output delays
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_A[*]}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_A[*]}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_BA[*]}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_BA[*]}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_DQ[*]}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_DQ[*]}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_DQML}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_DQML}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_DQMH}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_DQMH}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_nRAS}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_nRAS}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_nCAS}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_nCAS}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_nWE}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_nWE}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_nCS}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_nCS}]
set_output_delay -clock SDRAM_CLK -max 1.5 [get_ports {SDRAM_CKE}]
set_output_delay -clock SDRAM_CLK -min -0.8 [get_ports {SDRAM_CKE}]

# HDMI output delays
set_output_delay -clock HDMI_CLK -max 1.0 [get_ports {HDMI_TX_D[*]}]
set_output_delay -clock HDMI_CLK -min -0.5 [get_ports {HDMI_TX_D[*]}]
set_output_delay -clock HDMI_CLK -max 1.0 [get_ports {HDMI_TX_DE}]
set_output_delay -clock HDMI_CLK -min -0.5 [get_ports {HDMI_TX_DE}]
set_output_delay -clock HDMI_CLK -max 1.0 [get_ports {HDMI_TX_HS}]
set_output_delay -clock HDMI_CLK -min -0.5 [get_ports {HDMI_TX_HS}]
set_output_delay -clock HDMI_CLK -max 1.0 [get_ports {HDMI_TX_VS}]
set_output_delay -clock HDMI_CLK -min -0.5 [get_ports {HDMI_TX_VS}]

# HPS output delays
# These are handled by the HPS IP

#**************************************************************
# Set Clock Groups
#**************************************************************
set_clock_groups -asynchronous -group [get_clocks {FPGA_CLK1_50}] -group [get_clocks {SDRAM_CLK}]
set_clock_groups -asynchronous -group [get_clocks {FPGA_CLK1_50}] -group [get_clocks {HDMI_CLK}]

#**************************************************************
# Set False Path
#**************************************************************
# Asynchronous reset signals
set_false_path -from [get_ports {KEY[*]}] -to *
set_false_path -from [get_ports {SW[*]}] -to *

# LED outputs
set_false_path -from * -to [get_ports {LED_USER}]
set_false_path -from * -to [get_ports {LED_HDD}]
set_false_path -from * -to [get_ports {LED_POWER}]

#**************************************************************
# Set Multicycle Path
#**************************************************************
# None required for this design

#**************************************************************
# Set Maximum Delay
#**************************************************************
# None required for this design

#**************************************************************
# Set Minimum Delay
#**************************************************************
# None required for this design

#**************************************************************
# Set Input Transition
#**************************************************************
# None required for this design

#**************************************************************
# Set Load
#**************************************************************
# None required for this design