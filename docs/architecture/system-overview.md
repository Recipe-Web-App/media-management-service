# System Overview

## Purpose

The Media Management Service is a production-ready HTTP API built in Rust for handling file uploads, processing,
storage, and retrieval within a recipe web application ecosystem. It provides secure, efficient, and scalable media
handling with automatic optimization and content-addressable storage.

## High-Level Architecture

```text
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web Client    │───▶│  Load Balancer   │───▶│  Media Service  │
│   (Browser)     │    │   (Ingress)      │    │   (Axum API)    │
└─────────────────┘    └──────────────────┘    └────────┬────────┘
                                                         │
                                          ┌──────────────┼──────────────┐
                                          │              │              │
                                          ▼              ▼              ▼
                                 ┌───────────────┐ ┌──────────┐ ┌──────────────┐
                                 │  PostgreSQL   │ │  OAuth2  │ │ Prometheus   │
                                 │  (Metadata)   │ │ Service  │ │  (Metrics)   │
                                 └───────┬───────┘ └──────────┘ └──────────────┘
                                         │
┌─────────────────┐                     │
│   File System   │◀────────────────────┘
│  (CAS Storage)  │
└─────────────────┘
```

## Core Components

### 1. HTTP API Layer (Axum)

- **File Upload Endpoints**: Direct multipart uploads and presigned upload flow
- **File Download Endpoints**: Efficient streaming with range request support
- **Metadata Endpoints**: File information, processing status, pagination
- **Health Endpoints**: Kubernetes-compatible health and readiness checks
- **Metrics Endpoint**: Prometheus-format metrics for monitoring and observability
- **Authentication**: OAuth2 JWT validation for secure access control

### 2. Business Logic Layer (Domain)

- **Media Entities**: Core business objects for files, users, and metadata
- **Validation Rules**: File type, size, and content validation logic
- **Processing Rules**: Thumbnail generation, format conversion policies
- **Security Policies**: Access control and content verification

### 3. Data Layer (Infrastructure)

- **PostgreSQL Database**: Metadata storage with ACID transactions
- **Content-Addressable Storage**: Hash-based filesystem organization
- **OAuth2 Service**: External authentication and authorization service
- **Prometheus Metrics**: Performance monitoring and observability
- **External Integrations**: Logging and alerting systems

## Data Flow

### Authentication Flow

1. **Token Validation**: Extract JWT token from Authorization header
2. **Verification**: Validate token using shared secret or OAuth2 introspection
3. **Claims Extraction**: Extract user ID, scopes, and permissions from JWT
4. **Authorization**: Check user permissions for requested operation
5. **Context Injection**: Add user context to request for downstream handlers

### Upload Process

#### Direct Upload Flow

1. **Client Request**: Multipart file upload to `/media/` endpoint with JWT authentication
2. **Authentication**: Validate OAuth2 JWT token and extract user context
3. **Validation**: File type, size, and content validation
4. **Streaming Storage**: Content written to temporary location while computing hash
5. **Content Addressing**: File moved to final hash-based location
6. **Metadata Storage**: Database record created with file information and user ownership
7. **Response**: Return file metadata with media ID and content hash

#### Presigned Upload Flow (Recommended)

1. **Upload Request**: Client requests presigned upload URL with file metadata
2. **Authentication**: Validate OAuth2 JWT token
3. **Session Creation**: Generate secure upload token with HMAC signature
4. **Presigned URL**: Return time-limited, signed upload URL to client
5. **File Upload**: Client uploads file directly to presigned URL
6. **Validation**: Verify signature, expiration, and file size match
7. **Processing**: Hash computation and content-addressable storage
8. **Metadata Storage**: Update database with upload completion status

### Download Process

1. **Client Request**: GET request with media ID and JWT authentication
2. **Authentication**: Validate OAuth2 JWT token
3. **Authorization**: Verify user permissions for file access
4. **Database Lookup**: Retrieve media metadata and storage path
5. **File Retrieval**: Locate file in content-addressable storage
6. **Streaming Response**: Direct filesystem streaming with appropriate headers
7. **Caching**: Set cache headers for efficient subsequent requests

### Processing Pipeline (Future Enhancement)

Background media processing is planned for future implementation:

1. **Job Queue**: Async background processing jobs triggered by uploads
2. **Format Detection**: Analyze original file characteristics
3. **Variant Generation**: Create optimized formats (AVIF, WebP, thumbnails)
4. **Quality Assessment**: Verify generated variants meet quality standards
5. **Database Update**: Mark processing complete and variants available
6. **Cleanup**: Remove temporary files and failed attempts

**Current Status**: Files are stored as-is; processing pipeline is not yet implemented.

## Scalability Considerations

### Horizontal Scaling

- **Stateless Design**: All requests independent, no session state
- **Database Connection Pooling**: Efficient database resource utilization
- **File System Sharing**: NFS or distributed storage for multi-instance deployment
- **Processing Queue**: Separate processing workers for load distribution

### Performance Optimization

- **Content-Addressable Storage**: Automatic deduplication reduces storage needs
- **Aggressive Caching**: HTTP caching headers for reverse proxy optimization
- **Streaming I/O**: No memory buffering of large files
- **Async Processing**: Non-blocking I/O for concurrent request handling

### Monitoring and Observability

- **Structured Logging**: JSON logs with correlation IDs
- **Prometheus Metrics**: Comprehensive metrics export via `/metrics` endpoint
  - HTTP request/response metrics (duration, size, errors)
  - Business metrics (uploads, processing, storage)
  - System metrics (authentication attempts, rate limiting)
  - Configurable metric collection (enable/disable specific types)
- **Health Checks**: Kubernetes-compatible liveness and readiness probes
  - Database connectivity validation
  - Storage filesystem validation
  - Dependency health status
- **Distributed Tracing**: OpenTelemetry integration (planned)

## Security Model

### Input Security

- **File Type Validation**: Magic byte verification, not just extensions
- **Content Scanning**: Integration points for malware detection
- **Size Limits**: Configurable upload size restrictions
- **Rate Limiting**: Per-user and per-IP upload throttling

### Storage Security

- **Path Traversal Prevention**: Hash-based paths eliminate directory traversal
- **Content Verification**: SHA-256 checksums ensure file integrity
- **Sandboxing**: All file operations within defined base directories
- **Permission Model**: Separate read/write permissions for different operations

### Network Security

- **TLS Termination**: HTTPS-only communication
- **CORS Configuration**: Controlled cross-origin access
- **OAuth2 Authentication**: JWT token-based authentication
  - Offline JWT validation (fast, no network dependency)
  - Online token introspection (authoritative, real-time revocation)
  - Service-to-service authentication via Client Credentials Flow
- **Audit Logging**: Comprehensive access and modification logging

## Integration Points

### Recipe Application

- **User Context**: Integration with user authentication system
- **Recipe Attachments**: Link media files to recipe records
- **Search Integration**: Media metadata exposed for recipe search
- **Thumbnail Display**: Optimized images for recipe listings

### External Services

- **OAuth2 Service**: External authentication and authorization service
  - JWT token issuance and validation
  - User authentication and session management
  - Service-to-service authentication
- **Prometheus**: Metrics collection and monitoring
- **CDN Integration**: Content delivery network for global distribution (planned)
- **Backup Systems**: Integration with backup and disaster recovery (planned)
- **Log Aggregation**: Centralized logging with ELK or similar stack (planned)
