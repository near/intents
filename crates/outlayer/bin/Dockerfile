# Shared base: pre-built image with cargo-chef + Rust 1.86.
# Using lukemathwalker/cargo-chef avoids compiling chef itself (and its deps)
# against the pinned toolchain, which would fail if chef's own deps have a higher MSRV.
FROM lukemathwalker/cargo-chef:latest-rust-1.86-bookworm AS chef
WORKDIR /build

# Planner: snapshot the dependency graph into recipe.json.
# This layer only invalidates when Cargo.toml or Cargo.lock change.
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Builder stage 1: compile all dependencies using stub sources.
# --package keeps the scope to defuse-outlayer-bin's dep tree, avoiding
# build scripts in unrelated workspace crates (e.g. poa/factory compiles a WASM contract).
FROM chef AS builder
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json --package defuse-outlayer-bin

# Builder stage 2: compile the actual crate. Deps are already cached above,
# so this layer only re-runs when source files change.
COPY . .
RUN cargo build --release --package defuse-outlayer-bin

# Runtime: minimal image with just the binary.
# ca-certificates is needed for TLS connections to the NEAR RPC endpoint.
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/outlayer /usr/local/bin/outlayer
CMD ["outlayer"]
