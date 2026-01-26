# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Prerequisites

- **Rust 1.70+** - Latest stable Rust installation
- **PostgreSQL 14+** - Database for metadata storage
- **FFmpeg** - Required for video processing (system installation)
- **Pre-commit** - For automated code quality checks (`pip install pre-commit`)

## Development Commands

### Build & Run

```bash
cargo build                     # Build the project
cargo run                       # Run in local mode (uses .env.local)
cargo build --release           # Build optimized release version
RUN_MODE=production cargo run   # Run in production mode
```

### Testing

```bash
cargo test                              # Run all tests
cargo test --lib                        # Run only unit tests in src/
cargo test --test integration           # Run only integration tests
cargo test <test_name>                  # Run specific test by name
cargo test -- --nocapture               # Run tests with output visible
cargo test -- --test-threads=1          # Run tests sequentially (for DB tests)
RUST_LOG=debug cargo test -- --nocapture  # Run with debug logging
```

### Code Quality

```bash
cargo fmt --all                                            # Format code
cargo clippy --all-targets --all-features -- -D warnings   # Lint (warnings = errors)
cargo check                                                # Quick compile check
cargo deny check                                           # Security and license checks
pre-commit run --all-files                                 # Run all pre-commit hooks
```

### Code Coverage

```bash
cargo llvm-cov                    # Generate coverage report (80% minimum required)
cargo llvm-cov --html             # Generate HTML report in target/llvm-cov/html/
```

### Container Deployment (Minikube)

```bash
./scripts/containerManagement/deploy-container.sh      # Full deployment
./scripts/containerManagement/update-container.sh      # Rebuild and restart
./scripts/containerManagement/get-container-status.sh  # Check status
./scripts/containerManagement/cleanup-container.sh     # Remove resources
```

## Architecture Overview

This is a media management microservice for a recipe web application, built with **Clean Architecture** principles.

### Layer Structure

```
src/
├── main.rs              # Application entry point
├── lib.rs               # Library exports for testing
├── domain/              # Pure business logic (no external dependencies)
│   ├── entities/        # Core entities (Media, User)
│   ├── value_objects/   # Immutable types (ContentHash, MediaType, ProcessingStatus)
│   ├── repositories/    # Repository traits (interfaces)
│   └── services/        # Domain services
├── application/         # Use cases and orchestration
│   ├── use_cases/       # Business workflows (upload, list, delete, etc.)
│   ├── dto/             # Data transfer objects
│   └── ports/           # Port traits for external systems
├── infrastructure/      # External adapters
│   ├── persistence/     # PostgreSQL repository implementations
│   ├── storage/         # Filesystem storage with content-addressable paths
│   ├── http/            # HTTP server setup
│   ├── oauth2/          # OAuth2 authentication client
│   └── config/          # Configuration management
└── presentation/        # HTTP API layer
    ├── handlers/        # Endpoint handlers
    ├── middleware/      # Auth, metrics, rate limiting, error handling
    ├── routes/          # Route definitions
    └── extractors/      # Custom Axum extractors
```

### Key Design Decisions

Architecture Decision Records are in `docs/architecture/`:

- **ADR-001**: Axum web framework choice
- **ADR-002**: SQLx database toolkit
- **ADR-003**: Filesystem storage strategy
- **ADR-004**: Content-addressable storage (SHA-256 hash-based paths)
- **ADR-005**: Multi-format compression (AVIF/WebP/JPEG)

### Tech Stack

- **Axum 0.8** - Async web framework
- **Tokio** - Async runtime
- **SQLx 0.8** - Compile-time checked SQL with PostgreSQL
- **Tower-HTTP** - Production middleware (CORS, compression, tracing)
- **image-rs / ez-ffmpeg** - Media processing

## Runtime Modes

### Local Mode (Default)

- Loads config from `.env.local` + environment variables
- Storage: `./media`, `./media/temp` (relative paths)
- Logging: Pretty format

### Production Mode

- Config from environment variables only
- Storage: `/app/media`, `/app/media/temp` (absolute paths)
- Logging: JSON format
- Trigger: `RUN_MODE=production` or containerized deployment

## API Endpoints

Base URL: `http://localhost:3000/api/v1/media-management`

| Category           | Endpoints                                                 | Auth |
| ------------------ | --------------------------------------------------------- | ---- |
| Health             | `/health`, `/ready`, `/metrics`                           | None |
| Media CRUD         | `POST/GET/DELETE /media/`, `GET /media/{id}/download`     | JWT  |
| Presigned Upload   | `POST /media/upload-request`, `PUT /media/upload/{token}` | JWT  |
| Recipe Integration | `/media/recipe/{recipe_id}/*`                             | JWT  |

For detailed API documentation and examples, see `docs/api/` or import the Postman collection from `postman/`.

## Testing

### Test Organization

- **Unit tests**: `#[cfg(test)]` modules within source files
- **Integration tests**: `tests/` directory
- **Test utilities**: `tests/common/` (TestApp, builders, fixtures)

### Testing Patterns

```rust
// Use MediaBuilder for test entities
let media = MediaBuilder::new().with_filename("test.jpg").build();

// Use InMemoryMediaRepository for isolated repository tests
let repo = InMemoryMediaRepository::new();

// Use TestApp for HTTP endpoint testing (requires a Router)
let app = TestApp::new(router);
let response = app.get("/health").await;
response.assert_status(StatusCode::OK);
```

### Key Test Commands

```bash
cargo test oauth2                    # OAuth2 integration tests
cargo test metrics                   # Metrics endpoint tests
cargo test test_jwt_validation       # JWT validation tests
```

## Environment Configuration

Copy `.env.example` to `.env.local` for local development:

```bash
# Database (required)
POSTGRES_HOST=localhost
POSTGRES_PORT=5432
POSTGRES_DB=recipe_database
POSTGRES_SCHEMA=recipe_manager
MEDIA_MANAGEMENT_DB_USER=your-user
MEDIA_MANAGEMENT_DB_PASSWORD=your-password

# OAuth2 (required for authenticated endpoints)
OAUTH2_SERVICE_ENABLED=true
OAUTH2_SERVICE_BASE_URL=http://localhost:8080/api/v1/auth
JWT_SECRET=your-secret-key-at-least-32-characters

# Optional overrides
MEDIA_SERVICE_SERVER_PORT=3000
MEDIA_SERVICE_STORAGE_BASE_PATH=./media
```

## Code Quality Standards

The codebase enforces strict quality:

```rust
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![deny(warnings)]

// Allowed lints (configured in lib.rs)
#![allow(clippy::must_use_candidate)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_wrap)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::missing_errors_doc)]
```

- **Formatting**: 100 character line width (rustfmt.toml)
- **Coverage**: 80% minimum (enforced by pre-commit)
- **Pre-commit hooks**: Format, clippy, deny check, coverage
- **Conventional commits**: Enforced via pre-commit hook (e.g., `feat:`, `fix:`, `chore:`)

## Storage Strategy

Files are stored using **content-addressable storage**:

- Path derived from SHA-256 hash: `ab/cd/ef/abcdef123456...`
- Automatic deduplication (same content = same path)
- Prevents path traversal attacks
- Local: `./media/`, Container: `/app/media/`

## Kubernetes Deployment

Manifests in `k8s/` include deployment, service, PVC (50Gi), configmap/secret templates, network policy, and pod disruption budget.

Access via: `http://sous-chef-proxy.local/api/v1/media-management/`
