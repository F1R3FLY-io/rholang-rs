FROM rust:latest

# Install basic dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    openjdk-17-jdk \
    unzip \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Install Rust components
RUN rustup component add rustfmt clippy

# Install cargo tools
RUN cargo install cargo-audit cargo-tarpaulin

# Set up working directory
WORKDIR /app

# Set environment variables
ENV RUST_BACKTRACE=1

# Default command
CMD ["/bin/bash"]
