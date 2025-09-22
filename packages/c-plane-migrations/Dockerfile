# Build stage
FROM rust:1.88-slim as builder

# Install system dependencies for building
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace files for dependency resolution
COPY Cargo.toml Cargo.lock ./

# Copy all workspace member Cargo.toml files (required by workspace)
COPY packages/c-plane/Cargo.toml ./packages/c-plane/
COPY packages/c-plane-migrations/Cargo.toml ./packages/c-plane-migrations/
COPY packages/c-plane-agent/Cargo.toml ./packages/c-plane-agent/

# Create dummy source files for dependency caching (all workspace members)
RUN mkdir -p packages/c-plane/src packages/c-plane-migrations/src packages/c-plane-agent/src && \
    echo "fn main() {}" > packages/c-plane/src/main.rs && \
    echo "pub use sea_orm_migration::prelude::*;" > packages/c-plane-migrations/src/lib.rs && \
    echo "fn main() {}" > packages/c-plane-migrations/src/main.rs && \
    echo "fn main() {}" > packages/c-plane-agent/src/main.rs

# Build dependencies (this will be cached)
RUN cargo build --release --package c-plane-migrations

# Remove only the migrations dummy files and artifacts (keep other package dummy files)
RUN rm -rf packages/c-plane-migrations/src target/release/deps/c_plane_migrations*

# Copy actual migration source code
COPY packages/c-plane-migrations/src ./packages/c-plane-migrations/src

# Build the final binary
RUN cargo build --release --package c-plane-migrations

# Runtime stage
FROM debian:bookworm-slim as runtime

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -r -s /bin/false migrations

# Create app directory
WORKDIR /app

# Copy the migration binary from builder stage
COPY --from=builder /app/target/release/c-plane-migrations ./migrations

# Change ownership
RUN chown migrations:migrations /app/migrations

# Switch to non-root user
USER migrations

# Run migrations
CMD ["./migrations", "up"]