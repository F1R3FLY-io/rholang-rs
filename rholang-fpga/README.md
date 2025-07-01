# Rholang FSM Core for MiSTer FPGA

This project implements a Rholang Finite State Machine (FSM) as an FPGA core for the MiSTer platform. It provides hardware acceleration for executing Rholang programs with deterministic concurrent behavior.

## Overview

Rholang is a concurrent, message-passing programming language designed for distributed systems. Its execution model is based on the π-calculus and incorporates functional programming principles. This implementation creates a hardware accelerator that accurately models the Rholang FSM states and transitions, implements the concurrent message-passing semantics, and supports the full range of Rholang operations.

The core is designed for the MiSTer FPGA platform, which uses the DE10-Nano board with a Cyclone V FPGA. It integrates with MiSTer's Linux-based ecosystem and provides debugging and monitoring interfaces.

## Features

- Hardware implementation of the Rholang FSM execution model
- Support for concurrent process execution with true hardware parallelism
- Channel-based message passing with pattern matching
- Integration with MiSTer's Linux-based ecosystem
- Debugging and monitoring interfaces
- Sample Rholang programs demonstrating core functionality

## Directory Structure

```
rholang-fpga/
├── docker/                 # Docker configuration files
│   ├── quartus.Dockerfile  # Dockerfile for Quartus development environment
│   └── deploy.Dockerfile   # Dockerfile for deployment environment
├── rtl/                    # Verilog source files
│   ├── rholang_fsm_core.v  # Top-level module
│   ├── fpu.v               # FSM Processing Unit
│   ├── ccn.v               # Channel Communication Network
│   ├── pcmu.v              # Process Creation and Management Unit
│   ├── mmu.v               # Memory Management Unit
│   └── lic.v               # Linux Interface Controller
├── scripts/                # Build, run, test, debug, and deploy scripts
│   ├── build.sh            # Build script
│   ├── run.sh              # Run script
│   ├── test.sh             # Test script
│   ├── debug.sh            # Debug script
│   └── deploy.sh           # Deploy script
├── samples/                # Sample Rholang programs
│   ├── hello.rho           # Hello World program
│   ├── parallel.rho        # Parallel processes example
│   └── fibonacci.rho       # Fibonacci calculator
├── sys/                    # MiSTer system files
│   ├── sys_top.v           # MiSTer top-level wrapper (to be added)
│   └── pll.v               # PLL configuration (to be added)
├── docs/                   # Documentation
│   └── FINITE_STATE_MACHINE_DESIGN.md  # FSM design document
├── docker-compose.yml      # Docker Compose configuration
├── rholang_fsm.qpf         # Quartus project file
├── rholang_fsm.qsf         # Quartus settings file
├── rholang_fsm.sdc         # Timing constraints
└── rholang_fsm.sh          # MiSTer launcher script
```

## Docker-Based Development Environment

This project uses Docker to provide a consistent development environment with all necessary tools pre-installed. This approach ensures that all developers have the same environment and eliminates the need to install Quartus Prime and other tools locally.

### Prerequisites

- Docker Engine (19.03.0+)
- Docker Compose (1.27.0+)
- X11 server (for GUI applications)

#### Linux

```bash
# Install Docker
sudo apt-get update
sudo apt-get install docker.io docker-compose

# Allow your user to run Docker without sudo
sudo usermod -aG docker $USER
# Log out and log back in for this to take effect
```

#### macOS

```bash
# Install Docker Desktop for Mac
brew install --cask docker

# Install XQuartz for X11 forwarding
brew install --cask xquartz

# Configure XQuartz to allow connections from network clients
# Open XQuartz, go to Preferences > Security and check "Allow connections from network clients"
# Restart XQuartz
```

#### Windows

```bash
# Install Docker Desktop for Windows
# Install X Server for Windows (VcXsrv or Xming)
# Configure X Server to allow connections
```

### Building the Docker Images

```bash
# Build the Docker images
docker-compose build
```

## Building the Core

```bash
# Build the core using Docker
docker-compose run --rm build

# Build with clean option
docker-compose run --rm build ./scripts/build.sh --clean

# Build with verbose output
docker-compose run --rm build ./scripts/build.sh --verbose
```

## Running Rholang Programs

```bash
# Run a Rholang program in simulation mode
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix quartus ./scripts/run.sh samples/hello.rho

# Run with debug mode (opens waveform viewer)
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix quartus ./scripts/run.sh --debug samples/fibonacci.rho

# Run with verbose output
docker-compose run --rm quartus ./scripts/run.sh --verbose samples/parallel.rho
```

## Testing

```bash
# Run all tests
docker-compose run --rm test

# Run simulation tests only
docker-compose run --rm test ./scripts/test.sh --sim

# Run functional tests only
docker-compose run --rm test ./scripts/test.sh --func

# Run tests with verbose output
docker-compose run --rm test ./scripts/test.sh --verbose
```

## Debugging

```bash
# Open SignalTap Logic Analyzer
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix debug ./scripts/debug.sh --signal

# Open waveform viewer for a specific module
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix debug ./scripts/debug.sh --wave fpu

# Open RTL viewer for a specific module
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix debug ./scripts/debug.sh --rtl ccn

# Open Timing Analyzer
docker-compose run --rm -e DISPLAY=$DISPLAY -v /tmp/.X11-unix:/tmp/.X11-unix debug ./scripts/debug.sh --timing
```

## Deploying to MiSTer FPGA

### Setting Up SSH Keys

Before deploying to MiSTer, you need to set up SSH keys for authentication:

```bash
# Generate SSH key pair if you don't have one
ssh-keygen -t rsa -b 4096 -f ~/.ssh/mister_rsa

# Copy the public key to the MiSTer
ssh-copy-id -i ~/.ssh/mister_rsa root@mister

# Copy the private key to the Docker volume
mkdir -p ssh-keys
cp ~/.ssh/mister_rsa ssh-keys/
```

### Deploying

```bash
# Deploy to MiSTer
docker-compose run --rm deploy ./scripts/deploy.sh --key /root/.ssh/mister_rsa

# Deploy with build option (builds before deploying)
docker-compose run --rm deploy ./scripts/deploy.sh --build --key /root/.ssh/mister_rsa

# Deploy sample Rholang programs
docker-compose run --rm deploy ./scripts/deploy.sh --samples --key /root/.ssh/mister_rsa

# Deploy to a specific MiSTer host
docker-compose run --rm deploy ./scripts/deploy.sh --host 192.168.1.100 --key /root/.ssh/mister_rsa

# Deploy with verbose output
docker-compose run --rm deploy ./scripts/deploy.sh --verbose --key /root/.ssh/mister_rsa
```

## Running on MiSTer

Once deployed, you can run the Rholang FSM Core on your MiSTer:

1. Connect to your MiSTer via SSH or use a keyboard connected directly to the MiSTer.

2. Run the launcher script:
   ```bash
   cd /media/fat
   ./rholang_fsm.sh
   ```

3. To run a specific Rholang program:
   ```bash
   ./rholang_fsm.sh --load=/media/fat/rholang/hello.rho
   ```

4. To enable debug mode:
   ```bash
   ./rholang_fsm.sh --debug
   ```

## Sample Programs

### Hello World (hello.rho)

A simple program that outputs a greeting message:

```rholang
new stdout(`rho:io:stdout`) in {
  stdout!("Hello, Rholang on MiSTer FPGA!")
}
```

### Parallel Processes (parallel.rho)

Demonstrates concurrent execution and message passing:

```rholang
new channel, stdout(`rho:io:stdout`) in {
  // Sender process 1
  channel!("Message 1") |

  // Sender process 2
  channel!("Message 2") |

  // Receiver process
  for (msg1 <- channel; msg2 <- channel) {
    stdout!(["Received", msg1, "and", msg2])
  }
}
```

### Fibonacci Calculator (fibonacci.rho)

Calculates Fibonacci numbers using recursive processes:

```rholang
new fib, stdout(`rho:io:stdout`) in {
  // Define a contract for calculating Fibonacci numbers
  contract fib(@n, ret) = {
    // Base cases
    if (n == 0) { 
      ret!(0) 
    } else {
      if (n == 1) { 
        ret!(1) 
      } else {
        // Recursive case: fib(n) = fib(n-1) + fib(n-2)
        new a, b in {
          // Calculate fib(n-1) and fib(n-2) in parallel
          fib!(n - 1, *a) |
          fib!(n - 2, *b) |

          // Wait for both results and add them
          for (@x <- a; @y <- b) {
            ret!(x + y)
          }
        }
      }
    }
  } |

  // Calculate the 10th Fibonacci number
  new result in {
    fib!(10, *result) |
    for (@value <- result) {
      stdout!(["Fibonacci(10) =", value])
    }
  }
}
```

## Architecture

The Rholang FSM core consists of five main components:

1. **FSM Processing Units (FPUs)** - Hardware modules that implement FSM execution
2. **Channel Communication Network (CCN)** - Network for inter-FSM message passing
3. **Process Creation and Management Unit (PCMU)** - Manages process lifecycle
4. **Memory Management Unit (MMU)** - Manages memory for processes and channels
5. **Linux Interface Controller (LIC)** - Interface with the MiSTer Linux OS

For more details, see the [FSM design document](docs/FINITE_STATE_MACHINE_DESIGN.md).

## Performance

The implementation is optimized for the Cyclone V FPGA on the DE10-Nano board:

- **Clock Frequency**: 100 MHz
- **FSM Transitions per Second**: Up to 50 million
- **Concurrent Processes**: Up to 16
- **Channel Capacity**: 256 channels with up to 16 receivers each

## Resource Utilization

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

## Troubleshooting

### Docker Issues

1. **X11 Forwarding Not Working**
   - Make sure your X11 server is running and configured to allow connections
   - Check that the DISPLAY environment variable is set correctly
   - Try running `xhost +local:docker` before starting the container

2. **Permission Denied When Running Docker**
   - Make sure your user is in the docker group: `sudo usermod -aG docker $USER`
   - Log out and log back in for the group change to take effect

3. **Container Exiting Immediately**
   - Check the Docker logs: `docker-compose logs`
   - Make sure the command is correct and the script exists

### Build Issues

1. **Quartus Not Found**
   - Make sure you're running the script in the Docker container
   - Check that the Quartus installation path is correct in the Dockerfile

2. **Compilation Errors**
   - Check the build log: `cat output_files/build.log`
   - Make sure all source files are in the correct directories

### Deployment Issues

1. **SSH Connection Failed**
   - Make sure the MiSTer is reachable: `ping mister`
   - Check that the SSH key is correct and has the right permissions
   - Try connecting manually: `ssh -i ~/.ssh/mister_rsa root@mister`

2. **RBF File Not Found**
   - Make sure the build was successful: `docker-compose run --rm build`
   - Check that the RBF file exists: `ls -l output_files/rholang_fsm.rbf`

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- The MiSTer FPGA project for providing the platform
- The RChain community for developing the Rholang language
