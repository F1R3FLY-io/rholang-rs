FROM rust:nightly

# Install basic dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    curl \
    git \
    openjdk-17-jdk \
    unzip \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Ensure nightly toolchain and components
RUN rustup toolchain install nightly && \
    rustup default nightly && \
    rustup component add rustfmt clippy --toolchain nightly

# Install cargo tools
RUN cargo install cargo-audit cargo-tarpaulin

# Set up working directory
WORKDIR /app

# Set environment variables
ENV RUST_BACKTRACE=1 \
    RUSTUP_TOOLCHAIN=nightly

# Default command
CMD ["/bin/bash"]
