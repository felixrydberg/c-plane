# Multi-stage Dockerfile for c-plane package
# Designed to be built from workspace root with:
# docker build -f packages/c-plane/Dockerfile -t c-plane .

# Base stage with common dependencies
FROM rust:1.88-slim as base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory to workspace root
WORKDIR /workspace

# Copy workspace Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Copy all package Cargo.toml files (needed for workspace resolution)
COPY packages/c-plane/Cargo.toml ./packages/c-plane/
COPY packages/cplane-agent/Cargo.toml ./packages/cplane-agent/

# Development stage
FROM base as development

# Install development tools
RUN apt-get update && apt-get install -y \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reloading
RUN cargo install cargo-watch

# Copy all source code
COPY . .

# Build the c-plane package in debug mode
RUN cargo build -p c-plane

# Expose port
EXPOSE 8080

# Default command for development (with hot reload)
CMD ["cargo", "watch", "-x", "run", "-p", "c-plane"]

# Builder stage for production
FROM base as builder

# Create dummy projects to cache dependencies
RUN mkdir -p packages/c-plane/src packages/cplane-agent/src
RUN echo "fn main() {}" > packages/c-plane/src/main.rs
RUN echo "fn main() {}" > packages/cplane-agent/src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release -p c-plane && rm -rf packages/*/src target/release/deps/*c_plane*

# Copy actual source code for c-plane and its dependencies
COPY packages/c-plane/ ./packages/c-plane/
# Copy any shared packages that c-plane might depend on
# COPY packages/shared/ ./packages/shared/  # Example if you have shared code

# Build the c-plane application in release mode
RUN cargo build --release -p c-plane

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

# Copy the c-plane binary from builder stage
COPY --from=builder /workspace/target/release/c-plane ./c-plane

# Change ownership to appuser
RUN chown appuser:appuser /app/c-plane

# Switch to non-root user
USER appuser

# Expose port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

# Run the c-plane application
CMD ["./c-plane"]