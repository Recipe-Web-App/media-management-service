# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Development Commands

### Build & Run

- `cargo build` - Build the project
- `cargo run` - Build and run the application
- `cargo build --release` - Build optimized release version

### Deployment & Containers

#### Local Kubernetes Deployment

- `./scripts/containerManagement/deploy-container.sh` - Full deployment to local Minikube
- `./scripts/containerManagement/start-container.sh` - Start existing containers
- `./scripts/containerManagement/stop-container.sh` - Stop running containers
- `./scripts/containerManagement/update-container.sh` - Rebuild and update containers
- `./scripts/containerManagement/cleanup-container.sh` - Clean up all resources
- `./scripts/containerManagement/get-container-status.sh` - Check deployment status

#### Docker Commands

- `docker build -t media-management-service:latest .` - Build container image
- `eval "$(minikube docker-env)" && docker build -t media-management-service:latest .` - Build for Minikube

#### Kubernetes Access

- **Service URL**: `http://media-management.local/api/v1/media-management/`
- **Health Check**: `http://media-management.local/api/v1/media-management/health`
- **Readiness Check**: `http://media-management.local/api/v1/media-management/ready`

### Testing & Quality

- `cargo test` - Run all tests (unit and integration)
- `cargo test --lib` - Run only unit tests in src/ modules
- `cargo test --test integration` - Run only integration tests
- `cargo test -- --nocapture` - Run tests with output visible
- `cargo fmt --all` - Format all code according to rustfmt.toml configuration
- `cargo clippy --all-targets --all-features -- -D warnings` - Run linter with warnings as errors
- `cargo check` - Quick compile check without building executable

### Code Coverage

#### Primary Tool: cargo-llvm-cov (recommended)

- `cargo llvm-cov` - Generate code coverage report with summary
- `cargo llvm-cov --html` - Generate HTML coverage report in target/llvm-cov/html/
- `cargo llvm-cov --lcov --output-path target/llvm-cov/lcov.info` - Generate LCOV format for CI/CD
- `cargo llvm-cov --json --output-path target/llvm-cov/coverage.json` - Generate JSON report
- `cargo llvm-cov --workspace` - Include all workspace packages

#### Alternative Tool: cargo-tarpaulin

- `cargo tarpaulin` - Generate code coverage report (uses .tarpaulin.toml config)
- `cargo tarpaulin --out html` - Generate HTML coverage report in target/tarpaulin/
- `cargo tarpaulin --out xml` - Generate XML coverage report for CI/CD

**Coverage Requirements:**

- Target: 80% minimum line coverage
- Domain layer should achieve 90%+ coverage
- Infrastructure layer may have lower coverage due to external dependencies
- main.rs excluded from coverage (integration testing more appropriate)

### Pre-commit Integration

This project uses pre-commit hooks that automatically run:

- `cargo fmt --all` for code formatting
- `cargo clippy` with strict linting (warnings treated as errors)
- `cargo deny check` for license and dependency policy enforcement
- **`coverage-check`** - Enforces 80% minimum test coverage threshold using cargo-llvm-cov

Run `pre-commit run --all-files` to manually execute all hooks.

**Coverage Enforcement**:
The pre-commit hook checks that test coverage is at least 80%.
If coverage drops below this threshold, the commit is blocked.
You will receive guidance on how to improve coverage.

## Architecture Overview

This is a production-ready media management service for a recipe web application, built
with Rust and designed for Kubernetes deployment. The service handles file uploads,
processing, storage, and retrieval with a focus on security, performance, and scalability.

### Core Architecture Pattern: Clean/Hexagonal Architecture

The codebase follows Clean Architecture principles with clear separation of concerns:

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

### Tech Stack

#### Web Framework & Runtime

- **Axum 0.8** - Modern async web framework with excellent type safety and middleware support
- **Tokio** - Async runtime with built-in metrics and observability features
- **Tower-HTTP** - Production middleware (CORS, compression, rate limiting, tracing)

#### Database & Persistence

- **SQLx 0.8** - Compile-time checked SQL with async connection pooling
- **PostgreSQL** - Primary database for metadata and application state
- **Database Migrations** - Version-controlled schema evolution

#### Media Processing Pipeline

- **image-rs** - High-performance image manipulation and format conversion
- **ez-ffmpeg** - Safe Rust wrapper for video processing and thumbnail generation
- **Multi-format Support** - AVIF (primary), WebP (fallback), JPEG (legacy)

#### Storage Strategy

- **Content-Addressable Storage (CAS)** - Hash-based file organization for deduplication
- **Persistent Volume Storage** - Kubernetes-native persistent storage for production deployments
- **Filesystem Storage** - Direct file storage for optimal performance
- **Multi-tier Architecture** - Hot/warm/cold storage based on access patterns

#### Security & Validation

- **Input Validation** - Comprehensive file type and content validation
- **Path Traversal Prevention** - Secure file path handling and sandboxing
- **Content Verification** - SHA-256 checksums for integrity validation
- **Malware Scanning** - Integration points for virus detection

#### Observability & Monitoring

- **Tracing** - Structured logging with correlation IDs
- **OpenTelemetry** - Metrics, traces, and logs for production monitoring
- **Health Checks** - Kubernetes-native liveness and readiness probes
- **Prometheus Metrics** - Custom business metrics and system monitoring

### Configuration & Environment

- **Runtime Modes** - Local and production modes with automatic detection
- **Environment-based Configuration** - Dev/staging/production settings
- **Secret Management** - External secret injection for sensitive data
- **Feature Flags** - Runtime configuration for gradual feature rollouts
- **Validation** - Startup-time configuration validation

## Runtime Modes

The service supports two distinct runtime modes for different deployment scenarios:

### Local Mode (Development)

- **Trigger**: Default when no `RUN_MODE` set, or `RUN_MODE=local`
- **Configuration**: Loads from `.env.local` file + environment variables
- **Storage**: Relative paths (`./media`, `./media/temp`)
- **Logging**: Pretty format for readable development output
- **Use Case**: Local development, testing, debugging

### Production Mode (Deployment)

- **Trigger**: `RUN_MODE=production` or automatic in containerized environments
- **Configuration**: Environment variables only (no .env file dependency)
- **Storage**: Absolute container paths (`/app/media`, `/app/media/temp`)
- **Logging**: JSON format for structured log aggregation
- **Use Case**: Kubernetes deployment, production servers

### Environment Files

- **`.env.local`** - Local development configuration (copy from `.env.example`)
- **`.env.prod`** - Production deployment settings (used by deployment scripts)
- **`.env.example`** - Template with documentation and comments

### Development Setup

- **Strict Code Quality** - `#![deny(clippy::all)]`, `#![deny(clippy::pedantic)]`, `#![deny(warnings)]`
- **Consistent Formatting** - 100 character line width, 4-space indentation
- **Pre-commit Hooks** - Automated formatting, linting, and security checks
- **IDE Integration** - VS Code configuration with Rust Analyzer optimizations

### Project Structure (Current)

- `src/main.rs` - Application entry point with HTTP server
- `src/lib.rs` - Library root with clean architecture modules
- `Cargo.toml` - Dependencies and project configuration
- `rustfmt.toml` - Code formatting rules
- `.pre-commit-config.yaml` - Git hook configuration
- `CLAUDE.md` - This file (development guidance)

### API Structure

The service exposes HTTP endpoints following RESTful patterns:

**Base URL**: `http://localhost:3000/api/v1/media-management`

**Health Endpoints**:

- `GET /health` - Liveness probe
- `GET /ready` - Readiness probe

**Media Endpoints**:

- `POST /media/` - Upload media files
- `GET /media/` - List and search media
- `GET /media/{id}` - Get media metadata
- `GET /media/{id}/download` - Download media files

**Recipe-Related Media Endpoints**:

- `GET /media/recipe/{recipe_id}` - Get media IDs for a recipe
- `GET /media/recipe/{recipe_id}/ingredient/{ingredient_id}` - Get media IDs for a recipe ingredient
- `GET /media/recipe/{recipe_id}/step/{step_id}` - Get media IDs for a recipe step

The API follows the `/api/v1/media-management/` namespace pattern consistent with
other services in the recipe web application ecosystem.

## Development Notes

### Code Quality Standards

The codebase enforces extremely strict code quality standards with warnings treated
as errors. Always run `cargo clippy` before committing changes. The pre-commit hooks
will catch formatting and linting issues automatically.

### Architectural Principles

- **Domain-Driven Design** - Business logic drives the architecture
- **Dependency Inversion** - High-level modules don't depend on low-level modules
- **Interface Segregation** - Use specific, focused trait interfaces
- **Testability First** - Design for easy unit and integration testing

### Security Considerations

- **Input Validation** - Validate all user inputs at service boundaries
- **Path Security** - Use content-addressable storage to prevent path traversal
- **Content Verification** - Always verify file content matches expected types
- **Least Privilege** - Run with minimal required permissions

### Performance Guidelines

- **Async Everything** - Use async/await for all I/O operations
- **Stream Processing** - Handle large files with streaming to prevent memory exhaustion
- **Connection Pooling** - Use database connection pools for efficiency
- **Caching Strategy** - Implement multi-level caching for frequently accessed content

### Testing Framework

The project uses a comprehensive testing framework with the following structure:

#### Testing Dependencies

- `tokio-test` - Async testing utilities
- `mockall` - Mock generation for traits
- `rstest` - Parameterized and fixture-based testing
- `claims` - Better assertions for tests
- `tempfile` - Temporary file/directory testing
- `fake` - Data generation for tests
- `proptest` - Property-based testing

#### Test Organization

- **Unit Tests**: Located in `#[cfg(test)]` modules within each source file
- **Integration Tests**: Located in `tests/` directory
- **Test Utilities**: `tests/common/` contains shared testing infrastructure
- **Mock Implementations**: Repository and service mocks for isolated testing

#### Testing Guidelines

- Use `MediaBuilder` for creating test entities with sensible defaults
- Use `InMemoryMediaRepository` for testing repository logic without database
- Use `TestApp` for HTTP endpoint testing with proper assertions
- Write property-based tests for value objects to catch edge cases
- Mock external dependencies using `mockall` traits

## Container Deployment

### Quick Start

```bash
# Deploy to local Minikube
./scripts/containerManagement/deploy-container.sh

# Check deployment status
./scripts/containerManagement/get-container-status.sh

# Access the service
curl http://media-management.local/api/v1/media-management/health
```

### Container Management Scripts

| Script                    | Purpose                                                                      | When to Use                       |
| ------------------------- | ---------------------------------------------------------------------------- | --------------------------------- |
| `deploy-container.sh`     | **Full deployment** - Builds image, creates PVC, applies all K8s manifests   | Initial deployment, major updates |
| `start-container.sh`      | **Start service** - Scale deployment to 1 replica                            | After stopping, startup           |
| `stop-container.sh`       | **Stop service** - Scale deployment to 0 replicas                            | Maintenance, development pause    |
| `update-container.sh`     | **Update image** - Rebuild and restart with new code                         | Code changes, hot updates         |
| `cleanup-container.sh`    | **Interactive cleanup** - Remove all resources, optionally keep media files  | Clean slate, troubleshooting      |
| `get-container-status.sh` | **Status check** - Comprehensive deployment overview with storage monitoring | Monitoring, debugging             |

#### ⚠️ Important Storage Notes

- **Interactive PVC Cleanup**: `cleanup-container.sh` now prompts before deleting media files, allowing you to preserve data
- **Data Persistence**: Media files persist across pod restarts and deployments via Kubernetes Persistent Volumes
- **Storage Requirements**: Default allocation is 50Gi - ensure your cluster has sufficient storage capacity
- **First Deployment**: The deploy script automatically provisions persistent storage and waits for volume binding

#### Cleanup Script Options

The cleanup script supports different modes for automated use:

```bash
# Interactive mode (default) - prompts for PVC deletion
./scripts/containerManagement/cleanup-container.sh

# Keep media files safe - cleanup everything except PVC
./scripts/containerManagement/cleanup-container.sh --keep-pvc

# Delete everything including media files
./scripts/containerManagement/cleanup-container.sh --delete-pvc

# Force deletion without prompts (for CI/CD)
./scripts/containerManagement/cleanup-container.sh --force --delete-pvc
```

### Environment Configuration

The service automatically runs in **Production Mode** when containerized and loads configuration from environment variables:

**Required Environment Variables for Deployment:**

```bash
# Database Configuration (Required)
POSTGRES_HOST=your-postgres-host
POSTGRES_PORT=5432
POSTGRES_DB=recipe_database
POSTGRES_SCHEMA=recipe_manager
MEDIA_MANAGEMENT_DB_USER=your-db-user
MEDIA_MANAGEMENT_DB_PASSWORD=your-db-password

# Runtime Mode (Automatic in containers)
RUN_MODE=production

# Optional: Override defaults if needed
MEDIA_SERVICE_SERVER_HOST=0.0.0.0
MEDIA_SERVICE_SERVER_PORT=3000
MEDIA_SERVICE_STORAGE_BASE_PATH=/app/media
```

**Configure `.env.prod` for Deployment:**

The deployment scripts use `.env.prod` to substitute values into Kubernetes manifests:

```bash
# Copy and customize for your environment
cp .env.example .env.prod
# Edit with production database settings
vim .env.prod
```

### Kubernetes Deployment Structure

```text
k8s/
├── configmap-template.yaml    # Non-sensitive configuration
├── secret-template.yaml      # Database password only
├── persistentvolumeclaim.yaml # Persistent storage for media files ⭐ NEW
├── deployment.yaml           # Main service deployment (now uses PVC)
├── service.yaml             # ClusterIP service
├── ingress.yaml             # External access routing
├── networkpolicy.yaml       # Network security rules
└── poddisruptionbudget.yaml # High availability
```

### Security Features

- **Non-root container**: Runs as user `media` (UID 10001)
- **Read-only root filesystem**: Enhanced security posture
- **Network policies**: Restricts pod-to-pod communication
- **Resource limits**: Prevents resource exhaustion
- **Health checks**: Kubernetes liveness and readiness probes

### Storage Strategy

- **Persistent storage**: Uses Kubernetes Persistent Volume Claims (50Gi default allocation)
- **Content-addressable**: Hash-based file organization for deduplication
- **Data persistence**: Media files survive pod restarts, deployments, and service updates
- **Multi-tier support**: Configurable storage classes for different performance tiers

### Future Considerations

This media management service is part of a larger recipe web application ecosystem. The clean
architecture allows for:

- Easy extraction into a separate microservice
- Plugin-based storage backends (filesystem, S3, etc.)
- Horizontal scaling through stateless design
- Integration with other recipe app services through well-defined APIs
- Multi-environment deployment (dev/staging/production)
