# Media Management Service

A production-ready media management service built in Rust for handling file uploads, processing, storage, and
retrieval. Designed as part of a recipe web application ecosystem with a focus on performance, security, and
scalability.

## 🏗️ Architecture

This service follows **Clean Architecture** principles with a clear separation between domain logic, application use
cases, infrastructure adapters, and presentation layers. Built for **Kubernetes deployment** with comprehensive
observability and monitoring.

### Key Features

- **Content-Addressable Storage**: Hash-based file organization with automatic deduplication
- **Multi-Format Optimization**: Automatic AVIF/WebP conversion with fallback support
- **Security First**: Path traversal prevention, input validation, and content verification
- **Async Performance**: Built on Tokio with streaming file handling
- **Production Ready**: Comprehensive logging, metrics, and health checks

## 🚀 Tech Stack

### Core Framework

- **[Axum 0.8](https://github.com/tokio-rs/axum)** - Modern async web framework with excellent type safety
- **[Tokio](https://tokio.rs/)** - Async runtime with built-in metrics and observability
- **[SQLx 0.8](https://github.com/launchbadge/sqlx)** - Compile-time checked SQL with async connection pooling
- **[PostgreSQL](https://www.postgresql.org/)** - Primary database for metadata and application state

### Media Processing

- **[image-rs](https://github.com/image-rs/image)** - High-performance image manipulation and format conversion
- **[ez-ffmpeg](https://github.com/nathanbabcock/ez-ffmpeg)** - Safe Rust wrapper for video processing and thumbnails
- **Multi-format Support** - AVIF (primary), WebP (fallback), JPEG (legacy)

### Production Features

- **[OpenTelemetry](https://opentelemetry.io/)** - Distributed tracing and metrics
- **[Tracing](https://github.com/tokio-rs/tracing)** - Structured logging with correlation IDs
- **Content-Addressable Storage** - SHA-256 based file organization
- **Kubernetes Native** - Health checks, graceful shutdown, configurable deployments

## 📁 Project Structure

```text
src/
├── main.rs                 # Application entry point
├── lib.rs                  # Library root with public exports
├── domain/                 # Pure business logic (no external dependencies)
│   ├── entities/           # Core business entities (Media, User, etc.)
│   ├── value_objects/      # Immutable value types (FileHash, MediaType)
│   ├── repositories/       # Repository traits (interfaces)
│   └── services/          # Domain services
├── application/           # Use cases and orchestration
│   ├── use_cases/         # Application-specific business rules
│   ├── dto/               # Data transfer objects
│   └── ports/             # Port traits for external systems
├── infrastructure/        # External concerns (adapters)
│   ├── persistence/       # Database implementations
│   ├── storage/           # File storage adapters
│   ├── http/              # HTTP server setup
│   └── config/            # Configuration management
└── presentation/          # HTTP handlers and routing
    ├── handlers/          # HTTP route handlers
    ├── middleware/        # HTTP middleware
    ├── routes/            # Route definitions
    └── extractors/        # Custom Axum extractors
```

## 🛠️ Development

### Prerequisites

- **Rust 1.70+** - Latest stable Rust installation
- **PostgreSQL 14+** - Database for metadata storage
- **FFmpeg** - Required for video processing (system installation)
- **Pre-commit** - For automated code quality checks

### Local Development Setup

1. **Clone and enter directory**

   ```bash
   git clone <repository-url>
   cd media-management-service
   ```

2. **Install dependencies**

   ```bash
   cargo build
   ```

3. **Set up pre-commit hooks**

   ```bash
   pre-commit install
   ```

4. **Run the service**

   ```bash
   cargo run
   ```

### Development Commands

```bash
# Build and run
cargo build                    # Build the project
cargo run                      # Build and run the application
cargo build --release          # Build optimized release version

# Testing and quality
cargo test                     # Run all tests
cargo fmt --all               # Format code
cargo clippy --all-targets --all-features -- -D warnings  # Lint with warnings as errors
cargo check                   # Quick compile check

# Pre-commit hooks
pre-commit run --all-files    # Run all quality checks manually
```

## 📚 Documentation

### Architecture Documentation

- **[ADR-001](docs/architecture/ADR-001-web-framework-choice.md)** - Web Framework Choice (Axum)
- **[ADR-002](docs/architecture/ADR-002-database-toolkit-choice.md)** - Database Toolkit Choice (SQLx)
- **[ADR-003](docs/architecture/ADR-003-storage-strategy.md)** - Storage Strategy (Filesystem)
- **[ADR-004](docs/architecture/ADR-004-content-addressable-storage.md)** - Content-Addressable Storage
- **[ADR-005](docs/architecture/ADR-005-compression-strategy.md)** - Multi-Format Compression Strategy

### Development Guides

- **[CLAUDE.md](CLAUDE.md)** - Comprehensive development guidance for Claude Code
- **[docs/development/](docs/development/)** - Setup and testing guides (planned)
- **[docs/api/](docs/api/)** - API documentation (planned)

## 🌐 API Endpoints

### Health & Monitoring

- `GET /health` - Service health check (Kubernetes liveness probe)
- `GET /ready` - Service readiness check (Kubernetes readiness probe)

### Media Management API (v1)

Base URL: `http://localhost:3000/api/v1/media-management`

- `POST /media/` - Upload new media file
- `GET /media/` - List media files (with optional query parameters)
- `GET /media/{id}` - Get media metadata by ID
- `GET /media/{id}/download` - Download media file by ID

**Example Usage:**

```bash
# Health check
curl http://localhost:3000/health

# Upload media (multipart form-data)
curl -X POST http://localhost:3000/api/v1/media-management/media/ \
  -F "file=@image.jpg" \
  -F "filename=my-image.jpg"

# List media
curl http://localhost:3000/api/v1/media-management/media/

# Get media info
curl http://localhost:3000/api/v1/media-management/media/{media-id}

# Download media
curl http://localhost:3000/api/v1/media-management/media/{media-id}/download \
  -o downloaded-file.jpg
```

## 🔒 Security

- **Input Validation**: Comprehensive file type and content validation
- **Path Security**: Content-addressable storage prevents directory traversal
- **Content Verification**: SHA-256 checksums ensure file integrity
- **Sandboxing**: All file operations within defined safe directories
- **Least Privilege**: Minimal required permissions for operation

## 🚀 Deployment

Designed for **Kubernetes deployment** with:

- Health and readiness probes
- Graceful shutdown handling
- Configurable resource limits
- Horizontal pod autoscaling support
- Prometheus metrics export

See **[docs/deployment/](docs/deployment/)** for detailed deployment guides (planned).

## 🤝 Contributing

1. **Code Quality**: All code must pass `cargo clippy` with warnings as errors
2. **Testing**: Include tests for new functionality
3. **Documentation**: Update relevant documentation for changes
4. **Architecture**: Follow Clean Architecture principles and established patterns

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🔗 Related Projects

This service is part of a larger recipe web application ecosystem. The clean architecture design allows for easy
integration with other services and potential extraction as a standalone microservice.
