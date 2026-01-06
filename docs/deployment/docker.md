# Docker Deployment Guide

This guide covers building and running the Media Management Service using Docker containers.

## Overview

The service uses a **multi-stage Docker build** optimized for:

- **Small production images** (Debian Slim base)
- **Security hardening** (non-root user, read-only filesystem)
- **Efficient caching** (separate dependency and source builds)
- **Runtime dependencies** (FFmpeg for media processing)

## Dockerfile Architecture

### Multi-Stage Build Process

```dockerfile
# Stage 1: Builder (rust:1.92-bookworm)
- Installs Rust toolchain and build dependencies
- Compiles Rust application with optimizations
- Produces static binary

# Stage 2: Runtime (debian:bookworm-slim)
- Minimal runtime environment
- Installs only required system dependencies (FFmpeg, CA certificates)
- Creates non-root user for security
- Copies binary from builder stage
```

### Security Features

- **Non-root execution**: Runs as user `media` (UID 10001)
- **Read-only root filesystem**: Enhanced security posture
- **Minimal attack surface**: Only essential runtime dependencies
- **No package managers**: Production image has no apt/yum

## Building Images

### Local Development Build

```bash
# Build for local testing
docker build -t media-management-service:latest .

# Build with custom tag
docker build -t media-management-service:v1.0.0 .

# Build with build args (if needed)
docker build --build-arg RUST_VERSION=1.92 -t media-management-service:latest .
```

### Minikube Build (for Kubernetes)

```bash
# Set Docker environment to use Minikube's Docker daemon
eval "$(minikube docker-env)"

# Build image inside Minikube
docker build -t media-management-service:latest .

# Verify image exists in Minikube
docker images | grep media-management-service
```

### Production Build

```bash
# Build optimized production image
docker build --target runtime -t media-management-service:latest .

# Multi-platform build (if needed)
docker buildx build --platform linux/amd64,linux/arm64 -t media-management-service:latest .
```

## Running Containers

### Local Development

#### Using Docker Run

```bash
# Run with environment variables
docker run -d \
  --name media-management \
  -p 3000:3000 \
  -e RUN_MODE=production \
  -e POSTGRES_HOST=host.docker.internal \
  -e POSTGRES_DB=recipe_database \
  -e POSTGRES_SCHEMA=recipe_manager \
  -e MEDIA_MANAGEMENT_DB_USER=postgres \
  -e MEDIA_MANAGEMENT_DB_PASSWORD=password \
  -e OAUTH2_SERVICE_ENABLED=true \
  -e OAUTH2_CLIENT_ID=recipe-service-client \
  -e OAUTH2_CLIENT_SECRET=your_oauth2_secret \
  -e OAUTH2_SERVICE_BASE_URL=http://auth-service:8080/api/v1/auth \
  -e JWT_SECRET=your_jwt_secret_at_least_32_characters \
  -e OAUTH2_INTROSPECTION_ENABLED=false \
  -v $(pwd)/media:/app/media \
  media-management-service:latest
```

#### Using Docker Compose

Create `docker-compose.yml`:

```yaml
version: "3.8"

services:
  media-management:
    build: .
    ports:
      - "3000:3000"
    environment:
      - RUN_MODE=production
      - POSTGRES_HOST=postgres
      - POSTGRES_DB=recipe_database
      - POSTGRES_SCHEMA=recipe_manager
      - MEDIA_MANAGEMENT_DB_USER=postgres
      - MEDIA_MANAGEMENT_DB_PASSWORD=password
      - OAUTH2_SERVICE_ENABLED=true
      - OAUTH2_CLIENT_ID=recipe-service-client
      - OAUTH2_CLIENT_SECRET=your_oauth2_secret
      - OAUTH2_SERVICE_BASE_URL=http://auth-service:8080/api/v1/auth
      - JWT_SECRET=your_jwt_secret_at_least_32_characters
      - OAUTH2_INTROSPECTION_ENABLED=false
      - OAUTH2_SERVICE_TO_SERVICE_ENABLED=true
      - MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED=true
    volumes:
      - ./media:/app/media
    depends_on:
      - postgres
    healthcheck:
      test:
        [
          "CMD",
          "curl",
          "-f",
          "http://localhost:3000/api/v1/media-management/health",
        ]
      interval: 30s
      timeout: 10s
      retries: 3

  postgres:
    image: postgres:14
    environment:
      - POSTGRES_DB=recipe_database
      - POSTGRES_USER=postgres
      - POSTGRES_PASSWORD=password
    volumes:
      - postgres_data:/var/lib/postgresql/data
    ports:
      - "5432:5432"

volumes:
  postgres_data:
```

Run with Docker Compose:

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f media-management

# Stop services
docker-compose down
```

### Production Deployment

#### With External Database

```bash
# Run in production mode with external database
docker run -d \
  --name media-management-prod \
  --restart unless-stopped \
  -p 3000:3000 \
  -e RUN_MODE=production \
  -e POSTGRES_HOST=your-database-host.com \
  -e POSTGRES_DB=recipe_database \
  -e POSTGRES_SCHEMA=recipe_manager \
  -e MEDIA_MANAGEMENT_DB_USER=app_user \
  -e MEDIA_MANAGEMENT_DB_PASSWORD=secure_password \
  -e OAUTH2_SERVICE_ENABLED=true \
  -e OAUTH2_CLIENT_ID=recipe-service-client \
  -e OAUTH2_CLIENT_SECRET=production_oauth2_secret \
  -e OAUTH2_SERVICE_BASE_URL=https://auth.example.com/api/v1/auth \
  -e JWT_SECRET=production_jwt_secret_min_32_chars \
  -e OAUTH2_INTROSPECTION_ENABLED=false \
  -e MEDIA_SERVICE_LOGGING_LEVEL=info \
  -e MEDIA_SERVICE_LOGGING_FORMAT=json \
  -v /var/lib/media-management:/app/media \
  --read-only \
  --tmpfs /tmp \
  --security-opt no-new-privileges:true \
  media-management-service:latest
```

## Container Configuration

### Environment Variables

#### Required Variables

```bash
# Database Connection (required)
POSTGRES_HOST=database-host
POSTGRES_PORT=5432
POSTGRES_DB=recipe_database
POSTGRES_SCHEMA=recipe_manager
MEDIA_MANAGEMENT_DB_USER=username
MEDIA_MANAGEMENT_DB_PASSWORD=password

# OAuth2 Authentication (required)
OAUTH2_SERVICE_ENABLED=true
OAUTH2_CLIENT_ID=recipe-service-client
OAUTH2_CLIENT_SECRET=your_oauth2_client_secret
OAUTH2_SERVICE_BASE_URL=http://auth-service:8080/api/v1/auth
JWT_SECRET=your_jwt_secret_at_least_32_characters_long

# Runtime Mode (recommended)
RUN_MODE=production
```

#### Optional Variables

```bash
# Server Configuration
MEDIA_SERVICE_SERVER_HOST=0.0.0.0
MEDIA_SERVICE_SERVER_PORT=3000
MEDIA_SERVICE_SERVER_MAX_UPLOAD_SIZE=104857600

# OAuth2 Authentication Options
OAUTH2_INTROSPECTION_ENABLED=false  # Use offline JWT validation (faster)
OAUTH2_SERVICE_TO_SERVICE_ENABLED=true  # Enable service-to-service auth

# Metrics Configuration
MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED=true
MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_ENABLED=true

# Storage Configuration
MEDIA_SERVICE_STORAGE_BASE_PATH=/app/media
MEDIA_SERVICE_STORAGE_TEMP_PATH=/app/media/temp
MEDIA_SERVICE_STORAGE_MAX_FILE_SIZE=524288000

# Logging Configuration
MEDIA_SERVICE_LOGGING_LEVEL=info
MEDIA_SERVICE_LOGGING_FORMAT=json

# Database Pool Configuration
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=1
DATABASE_ACQUIRE_TIMEOUT_SECONDS=30
```

### Volume Mounts

#### Media Storage

```bash
# Persistent media storage
-v /host/media/path:/app/media

# With specific permissions
-v /host/media/path:/app/media:rw
```

#### Configuration Files

```bash
# Mount configuration file (if using file-based config)
-v /host/config/app.yaml:/app/config/app.yaml:ro
```

#### Logs (if needed)

```bash
# Mount log directory (usually not needed with container logging)
-v /host/logs:/app/logs
```

### Security Options

```bash
# Enhanced security options
docker run \
  --read-only \                    # Read-only root filesystem
  --tmpfs /tmp \                   # Writable /tmp in memory
  --security-opt no-new-privileges:true \  # Prevent privilege escalation
  --cap-drop ALL \                 # Drop all capabilities
  --cap-add NET_BIND_SERVICE \     # Only if binding to privileged ports
  --user 10001:10001 \            # Run as non-root user
  media-management-service:latest
```

## Health Checks

### Container Health Check

```dockerfile
# Add to Dockerfile
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
  CMD curl -f http://localhost:3000/api/v1/media-management/health || exit 1
```

### Manual Health Check

```bash
# Check container health
docker exec media-management curl -f http://localhost:3000/api/v1/media-management/health

# Check from host
curl -f http://localhost:3000/api/v1/media-management/health
```

## Monitoring & Logging

### Container Logs

```bash
# View logs
docker logs media-management

# Follow logs
docker logs -f media-management

# View recent logs
docker logs --tail 100 media-management

# View logs with timestamps
docker logs -t media-management
```

### Resource Monitoring

```bash
# Monitor resource usage
docker stats media-management

# Get container info
docker inspect media-management

# View container processes
docker exec media-management ps aux
```

### Log Aggregation

For production, integrate with log aggregation systems:

```bash
# Send logs to external system
docker run \
  --log-driver=fluentd \
  --log-opt fluentd-address=localhost:24224 \
  --log-opt tag=media-management \
  media-management-service:latest

# Or use syslog
docker run \
  --log-driver=syslog \
  --log-opt syslog-address=tcp://192.168.1.100:514 \
  media-management-service:latest
```

## Troubleshooting

### Build Issues

#### Dependency Build Failures

```bash
# Clean Docker build cache
docker builder prune

# Build without cache
docker build --no-cache -t media-management-service:latest .

# Build with verbose output
docker build --progress=plain -t media-management-service:latest .
```

#### Rust Compilation Errors

```bash
# Check Rust version in container
docker run --rm rust:1.92-bookworm rustc --version

# Build with specific Rust version
docker build --build-arg RUST_VERSION=1.92 -t media-management-service:latest .
```

### Runtime Issues

#### Container Won't Start

```bash
# Check container logs
docker logs media-management

# Run container interactively for debugging
docker run -it --entrypoint /bin/bash media-management-service:latest

# Check if binary exists and is executable
docker run --rm media-management-service:latest ls -la /app/media-management-service
```

#### Database Connection Issues

```bash
# Test database connectivity from container
docker exec media-management ping postgres-host

# Check environment variables
docker exec media-management env | grep POSTGRES

# Test PostgreSQL connection
docker exec media-management pg_isready -h $POSTGRES_HOST -p $POSTGRES_PORT
```

#### Permission Issues

```bash
# Check user and permissions
docker exec media-management id
docker exec media-management ls -la /app/

# Check volume mount permissions
docker exec media-management ls -la /app/media/
```

### Performance Issues

#### High Memory Usage

```bash
# Monitor memory usage
docker stats --no-stream media-management

# Check for memory leaks
docker exec media-management cat /proc/meminfo

# Limit memory usage
docker run --memory=512m media-management-service:latest
```

#### Slow Startup

```bash
# Profile startup time
time docker run --rm media-management-service:latest --version

# Check initialization logs
docker logs media-management | head -20
```

## Image Optimization

### Reduce Image Size

```dockerfile
# Use specific version tags
FROM rust:1.92-slim-bookworm

# Clean up in same layer
RUN apt-get update && \
    apt-get install -y ffmpeg && \
    rm -rf /var/lib/apt/lists/*

# Use .dockerignore to exclude unnecessary files
```

### Caching Optimization

```dockerfile
# Copy dependency files first for better caching
COPY Cargo.toml Cargo.lock ./
RUN cargo fetch

# Copy source code last
COPY src ./src
RUN cargo build --release
```

### Multi-Architecture Builds

```bash
# Setup buildx for multi-platform builds
docker buildx create --use

# Build for multiple architectures
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t media-management-service:latest \
  --push .
```

## Integration with CI/CD

### GitHub Actions Example

```yaml
name: Build and Push Docker Image

on:
  push:
    branches: [main]
    tags: ["v*"]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v2

      - name: Login to Container Registry
        uses: docker/login-action@v2
        with:
          registry: ghcr.io
          username: ${{ github.actor }}
          password: ${{ secrets.GITHUB_TOKEN }}

      - name: Build and push
        uses: docker/build-push-action@v4
        with:
          context: .
          platforms: linux/amd64,linux/arm64
          push: true
          tags: |
            ghcr.io/your-org/media-management-service:latest
            ghcr.io/your-org/media-management-service:${{ github.sha }}
```

## See Also

- [Kubernetes Deployment Guide](kubernetes.md)
- [Environment Setup Guide](../development/environment-setup.md)
- [Architecture Overview](../architecture/system-overview.md)
