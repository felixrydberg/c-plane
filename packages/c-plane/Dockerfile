# Multi-stage Dockerfile for c-plane package (standalone)
# Build from the package directory with:
# docker build -t c-plane .

# Base stage with common dependencies
FROM rust:1.88-slim as base

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy Cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Development stage
FROM base as development

# Install development tools
RUN apt-get update && apt-get install -y \
    git \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install cargo-watch for hot reloading
RUN cargo install cargo-watch

# Copy source code
COPY . .

# Build in debug mode
RUN cargo build

# Expose port
EXPOSE 8080

# Default command for development (with hot reload)
CMD ["cargo", "watch", "-x", "run"]

# Builder stage for production
FROM base as builder

# Create a dummy project to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies (this layer will be cached)
RUN cargo build --release && rm -rf src target/release/deps/c_plane*

# Copy source code
COPY src ./src

# Build the application in release mode
RUN cargo build --release

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