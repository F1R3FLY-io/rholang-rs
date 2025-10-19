# Build stage: compile the WASM package with wasm-pack using nightly toolchain
FROM rustlang/rust:nightly as builder

# Install wasm-pack
RUN cargo install wasm-pack --locked

# Ensure the wasm32 target is available on the preinstalled nightly toolchain
RUN rustup target add wasm32-unknown-unknown --toolchain nightly

ENV RUSTUP_TOOLCHAIN=nightly

WORKDIR /app

# Copy only necessary files to leverage Docker layer caching
COPY Cargo.toml Cargo.lock ./
COPY rholang-wasm-draft/Cargo.toml rholang-wasm-draft/Cargo.toml

# Copy the full workspace (kept after small manifests to leverage caching when possible)
COPY . .

# Build the wasm package targeting the web
WORKDIR /app/rholang-wasm-draft
RUN wasm-pack build --target web --out-dir www/pkg --release

# Runtime stage: serve the static files with Nginx
FROM nginx:alpine
COPY --from=builder /app/rholang-wasm-draft/www /usr/share/nginx/html

# Expose port and run nginx
EXPOSE 80
CMD ["nginx", "-g", "daemon off;"]
