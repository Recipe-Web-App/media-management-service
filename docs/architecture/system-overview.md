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
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                                         ▼
                                               ┌─────────────────┐
                                               │   PostgreSQL    │
                                               │   (Metadata)    │
                                               └─────────────────┘
                                                         │
┌─────────────────┐    ┌──────────────────┐             │
│   File System   │◀───│ Processing Queue │◀────────────┘
│  (CAS Storage)  │    │ (Background Jobs)│
└─────────────────┘    └──────────────────┘
```

## Core Components

### 1. HTTP API Layer (Axum)

- **File Upload Endpoints**: Chunked multipart uploads with streaming
- **File Download Endpoints**: Efficient streaming with range request support
- **Metadata Endpoints**: File information, processing status, search
- **Health Endpoints**: Kubernetes-compatible health and readiness checks

### 2. Business Logic Layer (Domain)

- **Media Entities**: Core business objects for files, users, and metadata
- **Validation Rules**: File type, size, and content validation logic
- **Processing Rules**: Thumbnail generation, format conversion policies
- **Security Policies**: Access control and content verification

### 3. Data Layer (Infrastructure)

- **PostgreSQL Database**: Metadata storage with ACID transactions
- **Content-Addressable Storage**: Hash-based filesystem organization
- **Processing Queue**: Async background job processing
- **External Integrations**: Monitoring, logging, and alerting systems

## Data Flow

### Upload Process

1. **Client Request**: Multipart file upload to `/upload` endpoint
2. **Validation**: File type, size, and content validation
3. **Streaming Storage**: Content written to temporary location while computing hash
4. **Content Addressing**: File moved to final hash-based location
5. **Metadata Storage**: Database record created with file information
6. **Background Processing**: Queue job for thumbnail/optimization generation
7. **Response**: Return file metadata and access URLs to client

### Download Process

1. **Client Request**: GET request with file hash or ID
2. **Authorization**: Verify user permissions for file access
3. **Content Negotiation**: Determine best format based on Accept headers
4. **Variant Selection**: Choose optimal file variant (AVIF, WebP, original)
5. **Streaming Response**: Direct filesystem streaming with appropriate headers
6. **Caching**: Set cache headers for efficient subsequent requests

### Processing Pipeline

1. **Job Queue**: Background processing jobs triggered by uploads
2. **Format Detection**: Analyze original file characteristics
3. **Variant Generation**: Create optimized formats (AVIF, WebP, thumbnails)
4. **Quality Assessment**: Verify generated variants meet quality standards
5. **Database Update**: Mark processing complete and variants available
6. **Cleanup**: Remove temporary files and failed attempts

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
- **Metrics Export**: Prometheus metrics for monitoring
- **Distributed Tracing**: OpenTelemetry integration for request tracing
- **Health Checks**: Kubernetes-compatible probe endpoints

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
- **Authentication Integration**: JWT or similar token-based auth
- **Audit Logging**: Comprehensive access and modification logging

## Integration Points

### Recipe Application

- **User Context**: Integration with user authentication system
- **Recipe Attachments**: Link media files to recipe records
- **Search Integration**: Media metadata exposed for recipe search
- **Thumbnail Display**: Optimized images for recipe listings

### External Services

- **CDN Integration**: Content delivery network for global distribution
- **Backup Systems**: Integration with backup and disaster recovery
- **Monitoring Stack**: Prometheus, Grafana, and alerting systems
- **Log Aggregation**: Centralized logging with ELK or similar stack
