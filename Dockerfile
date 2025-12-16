# Multi-stage Dockerfile for media-management-service
# Optimized for production deployment with security and performance considerations

# Build stage
FROM rust:1.92-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set up build environment
WORKDIR /usr/src/app

# Copy dependency files first for better layer caching
COPY Cargo.toml Cargo.lock ./

# Create dummy source to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached unless Cargo.toml changes)
RUN cargo build --release && rm -rf src target/release/deps/media_management_service*

# Copy actual source code
COPY src/ src/

# Build the actual application
RUN cargo build --release

# Runtime stage - minimal image
FROM debian:bookworm-slim

# Install runtime dependencies including media processing tools
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    curl \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user for security
RUN groupadd -r media && useradd -r -g media -s /bin/false media

# Create necessary directories for media storage
RUN mkdir -p /app/media /app/media/temp /app/logs && \
    chown -R media:media /app

# Copy binary from builder stage
COPY --from=builder /usr/src/app/target/release/media-management-service /usr/local/bin/media-management-service

# Set permissions
RUN chmod +x /usr/local/bin/media-management-service

# Switch to non-root user
USER media

# Set working directory
WORKDIR /app

# Expose the port the app runs on
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:3000/health || exit 1

# Environment variables with defaults
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1
ENV PORT=3000
ENV HOST=0.0.0.0

# Run the application
CMD ["media-management-service"]
