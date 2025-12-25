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

- **[OAuth2 Integration](https://oauth.net/2/)** - Full OAuth2 authentication with JWT validation and token introspection
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

2. **Configure environment**

   ```bash
   # Copy and customize local environment file
   cp .env.example .env.local
   # Edit .env.local with your local database settings
   ```

3. **Install dependencies**

   ```bash
   cargo build
   ```

4. **Set up pre-commit hooks**

   ```bash
   pre-commit install
   ```

5. **Run the service** (defaults to local mode)

   ```bash
   cargo run
   ```

### Run Modes

The service supports two runtime modes:

#### **Local Mode** (Default)

- **Configuration**: Loads from `.env.local` file + environment variables
- **Storage**: Uses relative paths (`./media`, `./media/temp`)
- **Logging**: Pretty format for readable development logs
- **Usage**: Automatic when no `RUN_MODE` set, or `RUN_MODE=local`

#### **Production Mode**

- **Configuration**: Environment variables only (no .env file loading)
- **Storage**: Uses absolute container paths (`/app/media`, `/app/media/temp`)
- **Logging**: JSON format for structured production logs
- **Usage**: Set `RUN_MODE=production` or deploy to Kubernetes

### Environment Files

- **`.env.local`** - Local development configuration (includes OAuth2 settings)
- **`.env.prod`** - Production deployment configuration (used by deployment scripts)
- **`.env.example`** - Template and documentation with OAuth2 configuration variables

**Key OAuth2 Configuration Variables:**

```bash
# OAuth2 Service Integration
OAUTH2_SERVICE_ENABLED=true
OAUTH2_CLIENT_ID=recipe-service-client
OAUTH2_CLIENT_SECRET=your-oauth2-client-secret-here
OAUTH2_SERVICE_BASE_URL=http://localhost:8080/api/v1/auth

# JWT Configuration (must match auth service)
JWT_SECRET=your-very-secure-secret-key-at-least-32-characters-long

# Authentication Features
OAUTH2_INTROSPECTION_ENABLED=false  # Use JWT validation (offline) vs API introspection (online)
OAUTH2_SERVICE_TO_SERVICE_ENABLED=true  # Enable service-to-service authentication
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

- `GET /api/v1/media-management/health` - Service health check (Kubernetes liveness probe)
- `GET /api/v1/media-management/ready` - Service readiness check (Kubernetes readiness probe)

### Media Management API (v1)

Base URL: `http://localhost:3000/api/v1/media-management`

**Authentication:** All media endpoints (except health checks) require OAuth2 JWT authentication via
`Authorization: Bearer {jwt_token}` header.

- `POST /media/` - Upload new media file
- `GET /media/` - List media files (with optional query parameters)
- `GET /media/{id}` - Get media metadata by ID
- `DELETE /media/{id}` - Delete media file and metadata
- `GET /media/{id}/download` - Download media file by ID

**Example Usage:**

```bash
# Health check (no auth required)
curl http://localhost:3000/api/v1/media-management/health

# Upload media (multipart form-data) - requires JWT token
curl -X POST http://localhost:3000/api/v1/media-management/media/ \
  -H "Authorization: Bearer <your-jwt-token>" \
  -F "file=@image.jpg" \
  -F "filename=my-image.jpg"

# List media - requires JWT token
curl -H "Authorization: Bearer <your-jwt-token>" \
  http://localhost:3000/api/v1/media-management/media/

# Get media info - requires JWT token
curl -H "Authorization: Bearer <your-jwt-token>" \
  http://localhost:3000/api/v1/media-management/media/{media-id}

# Delete media - requires JWT token
curl -X DELETE \
  -H "Authorization: Bearer <your-jwt-token>" \
  http://localhost:3000/api/v1/media-management/media/{media-id}

# Download media - requires JWT token
curl -H "Authorization: Bearer <your-jwt-token>" \
  http://localhost:3000/api/v1/media-management/media/{media-id}/download \
  -o downloaded-file.jpg
```

## 🔒 Security

- **OAuth2 Authentication**: Full OAuth2 integration with JWT token validation and introspection
- **Service-to-Service Auth**: OAuth2 Client Credentials Flow for microservice authentication
- **Input Validation**: Comprehensive file type and content validation
- **Path Security**: Content-addressable storage prevents directory traversal
- **Content Verification**: SHA-256 checksums ensure file integrity
- **Sandboxing**: All file operations within defined safe directories
- **Least Privilege**: Minimal required permissions for operation

## 🚀 Deployment

### Kubernetes Deployment

The service is designed for **Kubernetes deployment** with:

- Health and readiness probes
- Graceful shutdown handling
- Configurable resource limits
- Horizontal pod autoscaling support
- Prometheus metrics export

#### Quick Deployment

```bash
# Deploy to local Minikube
./scripts/containerManagement/deploy-container.sh

# Check deployment status
./scripts/containerManagement/get-container-status.sh

# Access service
curl http://sous-chef-proxy.local/api/v1/media-management/health
```

#### Container Management Scripts

| Script                    | Purpose                                                       |
| ------------------------- | ------------------------------------------------------------- |
| `deploy-container.sh`     | Full deployment to Minikube (builds image, applies manifests) |
| `start-container.sh`      | Start existing deployment (scale to 1 replica)                |
| `stop-container.sh`       | Stop deployment (scale to 0 replicas)                         |
| `update-container.sh`     | Rebuild image and restart deployment                          |
| `cleanup-container.sh`    | Remove all Kubernetes resources                               |
| `get-container-status.sh` | Show comprehensive deployment status                          |

#### Prerequisites

- **Minikube** - Local Kubernetes cluster
- **kubectl** - Kubernetes CLI tool
- **Docker** - Container runtime
- **jq** - JSON processing tool

#### Environment Configuration

The deployment uses `.env.prod` for environment variable substitution in Kubernetes manifests. Configure your
production settings in this file before deployment.

See **[docs/deployment/kubernetes.md](docs/deployment/kubernetes.md)** for detailed deployment guides.

## 🤝 Contributing

1. **Code Quality**: All code must pass `cargo clippy` with warnings as errors
2. **Testing**: Include tests for new functionality
3. **Documentation**: Update relevant documentation for changes
4. **Architecture**: Follow Clean Architecture principles and established patterns

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🚀 Recommended Enhancements

### **1. Media Processing & Optimization**

- **Image Processing Pipeline**: Add automatic format conversion (AVIF/WebP with JPEG fallback)
- **Thumbnail Generation**: Create multiple sizes (thumbnail, medium, large) for responsive display
- **Video Processing**: Implement video thumbnail extraction and format optimization
- **Metadata Extraction**: Extract EXIF data, dimensions, and media properties
- **Compression Optimization**: Smart quality adjustment based on content analysis

### **2. Advanced Storage Features**

- **Storage Backends**: Add S3/MinIO support alongside filesystem storage
- **CDN Integration**: CloudFront/CloudFlare integration for global media delivery
- **Backup & Replication**: Automatic backup to secondary storage locations
- **Storage Tiers**: Hot/warm/cold storage based on access patterns
- **Cleanup Jobs**: Background jobs for orphaned file removal

### **3. Performance & Scalability**

- **Caching Layer**: Redis/Memcached for metadata and frequently accessed content
- **Background Processing**: Async job queue (using something like `sqlx-queue` or external queue)
- **Database Read Replicas**: Support for read-only database connections
- **Connection Pooling**: Enhanced connection management with load balancing
- **Streaming Uploads**: Support for very large file uploads with resumable uploads

### **4. Enhanced Security**

- **Virus Scanning**: Integration with ClamAV or cloud-based malware detection
- **Content Moderation**: AI-based content filtering for inappropriate material
- **Access Control**: Role-based permissions and fine-grained access control
- **Rate Limiting**: Per-user and per-IP rate limiting with Redis backend
- **Audit Logging**: Comprehensive audit trails for all operations

### **5. Advanced API Features**

- **Batch Operations**: Bulk upload, delete, and metadata update endpoints
- **Search Functionality**: Full-text search with indexing (potentially Elasticsearch)
- **Filtering & Sorting**: Advanced query parameters for media listing
- **Versioning**: Support for multiple versions of the same media file
- **Tagging System**: User-defined tags and categories for media organization

### **6. Monitoring & Analytics**

- **Metrics Dashboard**: Grafana dashboard for service metrics
- **Business Intelligence**: Usage analytics, storage statistics, performance metrics
- **Alerting**: Proactive alerts for service degradation or errors
- **Request Tracing**: Distributed tracing with Jaeger/Zipkin
- **Performance Profiling**: Built-in profiling endpoints for production debugging

### **7. Developer Experience**

- **OpenAPI Specification**: Auto-generated API documentation with examples
- **SDK Generation**: Client libraries for popular languages
- **Admin Interface**: Web UI for service administration and monitoring
- **Migration Tools**: Database migration utilities and rollback support
- **Load Testing**: Built-in performance testing tools and benchmarks

### **8. Integration Features**

- **Webhook Support**: Configurable webhooks for processing events
- **External Authentication**: OAuth2/OIDC integration for enterprise SSO
- **Message Queue Integration**: Kafka/RabbitMQ for event-driven architecture
- **Recipe Service Integration**: Enhanced integration with recipe metadata
- **Third-party Integrations**: Support for external media services and APIs

### **9. Operational Excellence**

- **Blue-Green Deployments**: Zero-downtime deployment strategies
- **Feature Flags**: Runtime feature toggling without redeployment
- **Configuration Hot Reload**: Dynamic configuration updates
- **Graceful Shutdown**: Enhanced shutdown procedures with proper cleanup
- **Resource Management**: Dynamic resource allocation based on load

### **10. Data Management**

- **Media Analytics**: Track view counts, access patterns, and usage statistics
- **Data Retention Policies**: Automatic cleanup of old or unused media
- **Import/Export Tools**: Bulk data migration utilities
- **Database Optimization**: Query optimization and index management
- **Backup Automation**: Scheduled backups with retention policies

### **Priority Recommendations**

**High Priority (Immediate Value):**

1. **Image Processing Pipeline** - Core functionality enhancement
2. **Caching Layer** - Significant performance improvement
3. **Background Processing** - Better scalability and user experience
4. **OpenAPI Documentation** - Improved developer experience

**Medium Priority (Strategic Value):**

1. **S3 Storage Backend** - Cloud deployment flexibility
2. **Enhanced Security** (virus scanning, rate limiting)
3. **Monitoring Dashboard** - Operational visibility
4. **Batch Operations** - API completeness

**Lower Priority (Future Enhancement):**

1. **Advanced Analytics** - Business intelligence
2. **Admin Interface** - Operational convenience
3. **SDK Generation** - Ecosystem expansion

## 🔗 Related Projects

This service is part of a larger recipe web application ecosystem. The clean architecture design allows for easy
integration with other services and potential extraction as a standalone microservice.
