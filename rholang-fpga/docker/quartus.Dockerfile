# Base image with Ubuntu 20.04
FROM ubuntu:20.04

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies for Quartus Prime
RUN apt-get update && apt-get install -y \
    wget \
    unzip \
    libglib2.0-0 \
    libsm6 \
    libxi6 \
    libxrender1 \
    libxrandr2 \
    libfreetype6 \
    libfontconfig1 \
    libxext6 \
    libxcursor1 \
    libxfixes3 \
    libxinerama1 \
    libgl1-mesa-glx \
    libgl1-mesa-dri \
    libncurses5 \
    libc6-dev \
    make \
    gcc \
    g++ \
    python3 \
    python3-pip \
    git \
    ssh \
    rsync \
    xterm \
    x11-apps \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Python packages
RUN pip3 install pytest pytest-xvfb

# Create directory for Quartus installation
RUN mkdir -p /opt/quartus

# Download and install Quartus Prime Lite (this is a placeholder - actual installation would be more complex)
# In a real scenario, you would either:
# 1. Download from Intel's website (requires login)
# 2. Use a local copy of the installer
# 3. Use a pre-built Docker image with Quartus already installed
RUN echo "In a real implementation, Quartus Prime would be installed here." > /opt/quartus/README.txt

# Add Quartus to PATH
ENV PATH="/opt/quartus/quartus/bin:${PATH}"

# Create workspace directory
WORKDIR /workspace

# Add scripts to path
ENV PATH="/workspace/scripts:${PATH}"

# Default command
CMD ["/bin/bash"]