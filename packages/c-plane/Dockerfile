FROM rust:1.88-slim as base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy workspace Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy all package Cargo.toml files for workspace dependency resolution
COPY packages/c-plane/Cargo.toml ./packages/c-plane/

# Development stage
FROM base as development

# Install development tools and cargo-watch for hot reloading
RUN apt-get update && apt-get install -y \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for development hot reloading
RUN cargo install cargo-watch

# Create dummy src files for dependency caching
RUN mkdir -p packages/c-plane/src && \
    echo "fn main() {}" > packages/c-plane/src/main.rs

# Build dependencies ONLY (this layer will be cached unless Cargo.toml changes)
RUN cargo build --package c-plane

# Remove dummy files and the dummy binary (but keep dependencies compiled)
RUN rm -rf packages/c-plane/src target/debug/deps/c_plane* target/debug/c-plane*

# Copy actual source code for c-plane
COPY packages/c-plane/src ./packages/c-plane/src

# Pre-build the application in debug mode (this layer caches unless source changes)
RUN cargo build --package c-plane

# Expose port
EXPOSE 8080

# Use cargo-watch for instant recompilation on file changes
CMD ["cargo", "watch", "--package", "c-plane", "-x", "run --package c-plane"]

# Builder stage for production
FROM base as builder

# Create dummy src files for dependency caching
RUN mkdir -p packages/c-plane/src && \
    echo "fn main() {}" > packages/c-plane/src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release --package c-plane && \
    rm -rf packages/c-plane/src target/release/deps/c_plane*

# Copy actual source code for c-plane
COPY packages/c-plane/src ./packages/c-plane/src

# Build the application in release mode
RUN cargo build --release --package c-plane

# Production stage
FROM debian:bookworm-slim as production

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create a non-root user
RUN useradd -r -s /bin/false appuser

# Create app directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/c-plane ./c-plane

# Change ownership to appuser
RUN chown appuser:appuser /app/c-plane

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the application
CMD ["./c-plane"]