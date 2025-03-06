FROM rust:1.72 as builder

WORKDIR /app

# Copy the local cargo configuration files
COPY .cargo /app/.cargo
COPY Cargo.toml Cargo.lock rust-toolchain /app/
COPY intents/runesdex-solver/Cargo.toml /app/intents/runesdex-solver/
COPY intents/core/Cargo.toml /app/intents/core/

# Create dummy source files to build dependencies
RUN mkdir -p /app/src && \
    echo "fn main() {}" > /app/src/main.rs && \
    mkdir -p /app/intents/runesdex-solver/src && \
    echo "fn main() {}" > /app/intents/runesdex-solver/src/main.rs && \
    echo "pub fn dummy() {}" > /app/intents/runesdex-solver/src/lib.rs && \
    mkdir -p /app/intents/core/src && \
    echo "pub fn dummy() {}" > /app/intents/core/src/lib.rs

# Build dependencies only
RUN cargo build --release --bin runesdex-solver

# Copy the actual source code
COPY intents/core /app/intents/core
COPY intents/runesdex-solver /app/intents/runesdex-solver

# Build the application
RUN cargo build --release --bin runesdex-solver

# Create the runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/runesdex-solver /usr/local/bin/

# Set up environment variables
ENV RUST_LOG=info
ENV RUNESDEX_API_KEY=""
ENV NEAR_ACCOUNT_ID=""
ENV NEAR_PRIVATE_KEY=""
ENV SOLVER_BUS_URL="wss://solver-relay-v2.chaindefuser.com/ws"

ENTRYPOINT ["runesdex-solver"] 