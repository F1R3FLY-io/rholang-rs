# Base image with Ubuntu 20.04
FROM ubuntu:20.04

# Avoid interactive prompts during package installation
ENV DEBIAN_FRONTEND=noninteractive

# Install dependencies for deployment
RUN apt-get update && apt-get install -y \
    openssh-client \
    rsync \
    sshpass \
    curl \
    wget \
    python3 \
    python3-pip \
    git \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# Install Python packages for deployment
RUN pip3 install paramiko scp

# Create .ssh directory for SSH keys
RUN mkdir -p /root/.ssh && chmod 700 /root/.ssh

# Create workspace directory
WORKDIR /workspace

# Add scripts to path
ENV PATH="/workspace/scripts:${PATH}"

# Default command
CMD ["/bin/bash"]