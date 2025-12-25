# Media Management Service API Documentation

## Base URL

- **Local Development**: `http://localhost:3000/api/v1/media-management`
- **Kubernetes**: `http://sous-chef-proxy.local/api/v1/media-management`

## Overview

The Media Management Service is a production-ready microservice for handling media file uploads, processing, storage, and
retrieval within the Recipe Web Application ecosystem. Built with Rust using Axum framework and following
Clean/Hexagonal Architecture principles.

## Authentication

The service uses **OAuth2 JWT authentication** for all media endpoints (health checks remain public).

### Required Header

All authenticated endpoints require the following header:

```http
Authorization: Bearer {jwt_token}
```

### JWT Token Format

Tokens must be valid OAuth2 JWT tokens with the following claims structure:

```json
{
  "iss": "oauth2-service-url", // Issuer
  "aud": ["media-management-service"], // Audience
  "sub": "user-id-12345", // Subject (User ID)
  "client_id": "recipe-service-client", // OAuth2 Client ID
  "scopes": ["media:read", "media:write"], // OAuth2 Scopes
  "type": "user", // Token Type
  "exp": 1234567890, // Expiration
  "iat": 1234567800, // Issued At
  "nbf": 1234567800, // Not Before
  "jti": "token-unique-id" // JWT ID
}
```

### Authentication Error Responses

**401 Unauthorized:**

```json
{
  "error": "Unauthorized",
  "message": "Invalid or missing authentication token"
}
```

### OAuth2 Configuration

- **JWT Validation**: Supports offline validation (default) and online introspection
- **Service-to-Service**: OAuth2 Client Credentials Flow for microservice auth
- **Token Caching**: Performance optimization with configurable TTL

---

## Health & Status Endpoints

### Health Check

**GET** `/health`

Kubernetes liveness probe endpoint with comprehensive dependency validation.

**Responses:**

**Healthy (all dependencies operational):**

```json
{
  "status": "healthy",
  "timestamp": "2025-01-15T10:30:00Z",
  "service": "media-management-service",
  "version": "0.1.0",
  "response_time_ms": 25,
  "checks": {
    "database": {
      "status": "healthy",
      "response_time_ms": 5
    },
    "storage": {
      "status": "healthy",
      "response_time_ms": 3
    },
    "overall": "healthy"
  }
}
```

**Degraded (some dependencies working):**

```json
{
  "status": "degraded",
  "timestamp": "2025-01-15T10:30:00Z",
  "service": "media-management-service",
  "version": "0.1.0",
  "response_time_ms": 2050,
  "checks": {
    "database": {
      "status": "unhealthy",
      "response_time_ms": 2000
    },
    "storage": {
      "status": "healthy",
      "response_time_ms": 3
    },
    "overall": "degraded"
  }
}
```

**Status Codes:**

- `200 OK` - Service is healthy or degraded (can still serve some requests)
- `503 Service Unavailable` - Service is unhealthy (all dependencies failed)

---

### Readiness Check

**GET** `/ready`

Kubernetes readiness probe endpoint with binary ready/not-ready status.

**Responses:**

**Ready (all dependencies operational):**

```json
{
  "status": "ready",
  "timestamp": "2025-01-15T10:30:00Z",
  "service": "media-management-service",
  "version": "0.1.0",
  "response_time_ms": 25,
  "checks": {
    "database": {
      "status": "ready",
      "response_time_ms": 5
    },
    "storage": {
      "status": "ready",
      "response_time_ms": 3
    },
    "overall": "ready"
  }
}
```

**Not Ready (any dependency failed):**

```json
{
  "status": "not_ready",
  "timestamp": "2025-01-15T10:30:00Z",
  "service": "media-management-service",
  "version": "0.1.0",
  "response_time_ms": 2010,
  "checks": {
    "database": {
      "status": "timeout",
      "response_time_ms": 2000
    },
    "storage": {
      "status": "ready",
      "response_time_ms": 3
    },
    "overall": "not_ready"
  }
}
```

**Key Differences from Health Check:**

- **Binary status**: Either "ready" or "not_ready" (no "degraded" state)
- **Traffic routing**: Used by Kubernetes to decide if pod should receive traffic
- **Stricter criteria**: ALL dependencies must be operational for "ready" status

**Status Codes:**

- `200 OK` - Service is ready to accept traffic (all dependencies operational)
- `503 Service Unavailable` - Service is not ready (any dependency failed)

---

## Monitoring Endpoints

### Metrics

**GET** `/metrics`

Provides Prometheus-compatible metrics for monitoring and observability.

**Authentication**: None (monitoring endpoints are typically unauthenticated)

**Response Format**: Prometheus text format

**Metrics Categories**:

- **HTTP Request Metrics**:
  - `http_requests_total` - Total number of HTTP requests by method, route, and status
  - `http_request_duration_seconds` - Request duration histogram
  - `http_request_size_bytes` - Request size counter
  - `http_response_size_bytes` - Response size counter

- **Business Metrics**:
  - `media_uploads_total` - Total media file uploads
  - `media_processing_duration_seconds` - Media processing time histogram
  - `media_storage_bytes_total` - Total storage space used

- **System Metrics**:
  - `http_errors_total` - HTTP error counter by status code
  - `auth_attempts_total` - Authentication attempts by outcome
  - `rate_limit_exceeded_total` - Rate limiting violations

- **Error Metrics**:
  - Error rates by endpoint and type
  - Failed request classifications

**Configuration**: Controlled by environment variables:

- `MEDIA_SERVICE_MIDDLEWARE_METRICS_ENABLED` - Enable/disable metrics collection
- `MEDIA_SERVICE_MIDDLEWARE_METRICS_ENDPOINT_ENABLED` - Enable/disable `/metrics` endpoint
- `MEDIA_SERVICE_MIDDLEWARE_METRICS_*` - Fine-grained control over metric types

**Example Response**:

```prometheus
# HELP http_requests_total Total number of HTTP requests
# TYPE http_requests_total counter
http_requests_total{method="GET",route="/health",status="200"} 42

# HELP http_request_duration_seconds HTTP request duration in seconds
# TYPE http_request_duration_seconds histogram
http_request_duration_seconds_bucket{method="GET",route="/health",status="200",le="0.1"} 40
http_request_duration_seconds_bucket{method="GET",route="/health",status="200",le="0.5"} 42

# HELP media_uploads_total Total number of media uploads
# TYPE media_uploads_total counter
media_uploads_total{status="success"} 15
media_uploads_total{status="failed"} 2
```

**Status Codes**:

- `200 OK` - Metrics data returned successfully
- `404 Not Found` - Metrics endpoint is disabled in configuration

**Example Usage**:

```bash
# Get metrics data
curl http://localhost:3000/metrics

# Using Kubernetes service URL
curl http://sous-chef-proxy.local/metrics

# Prometheus scrape configuration
scrape_configs:
  - job_name: 'media-management-service'
    static_configs:
      - targets: ['sous-chef-proxy.local:80']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

**Integration with Monitoring**:

- **Prometheus**: Direct scraping support
- **Grafana**: Pre-built dashboard compatible
- **Kubernetes**: Works with Prometheus Operator and ServiceMonitor
- **Alerting**: Metric thresholds for operational alerts

---

## Media Endpoints

### Upload Media

**POST** `/media/`

Upload a new media file to the system with automatic content-addressable storage and deduplication.

**Request Headers:**

- `Content-Type: multipart/form-data` (required)

**Request Body:**

Multipart form data with the following fields:

- `file` (required): The file to upload
  - Must include filename in the multipart field
  - Content-Type is automatically detected from file content
  - Supported formats: JPEG, PNG, WebP, AVIF, GIF, MP4, WebM
- `filename` (optional): Alternative way to specify filename if not in file field

**File Size Limits:**

- Default: Configurable via `max_file_size` (typically 10MB)
- Files exceeding the limit will be rejected with 400 Bad Request

**Example Request:**

```bash
curl -X POST "http://localhost:3000/api/v1/media-management/media/" \
  -H "Authorization: Bearer <your-jwt-token>" \
  -F "file=@example.jpg;type=image/jpeg"
```

**Successful Response:**

```json
{
  "media_id": 123,
  "content_hash": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "processing_status": "Pending",
  "upload_url": null
}
```

**Response Fields:**

- `media_id`: Unique database-assigned identifier (integer)
- `content_hash`: SHA-256 hash of file content (64-character hex string)
- `processing_status`: Current processing status (`"Pending"`, `"Processing"`, `"Complete"`, `"Failed"`)
- `upload_url`: Direct access URL (currently null, reserved for future use)

**Error Responses:**

**400 Bad Request - No file data:**

```json
{
  "error": "Bad Request",
  "message": "No file data provided"
}
```

**400 Bad Request - File too large:**

```json
{
  "error": "Bad Request",
  "message": "File too large: exceeds maximum size limit of 10485760 bytes"
}
```

**400 Bad Request - Invalid content type:**

```json
{
  "error": "Bad Request",
  "message": "Content type validation failed: unsupported file format"
}
```

**500 Internal Server Error - Storage/Database failure:**

```json
{
  "error": "Internal Server Error",
  "message": "Failed to save media metadata: database connection failed"
}
```

**Status Codes:**

- `200 OK` - File uploaded successfully (includes deduplication cases)
- `400 Bad Request` - Invalid request (missing file, too large, unsupported format)
- `500 Internal Server Error` - Server-side failure (database, storage issues)

**Content Deduplication:**

The service implements automatic content deduplication:

1. **Hash Calculation**: SHA-256 hash computed for uploaded file content
2. **Duplicate Detection**: If hash already exists in database, existing media is returned
3. **Storage Optimization**: Duplicate files are not stored again
4. **Response Consistency**: Same response format whether file is new or duplicate

**Content-Addressable Storage:**

Files are stored using content-addressable paths:

- **Path Format**: `{first_2_hash_chars}/{next_2_chars}/{next_2_chars}/{full_hash}`
- **Example**: `ab/cd/ef/abcdef123456...`
- **Benefits**: Natural deduplication, efficient retrieval, path predictability

**Processing Status Flow:**

1. **Upload Complete** → Status: `"Pending"`
2. **Future**: Async processing → Status: `"Processing"`
3. **Future**: Processing complete → Status: `"Complete"`
4. **Future**: Processing failed → Status: `"Failed"`

---

## Presigned Upload Endpoints

### Initiate Presigned Upload Session

**POST** `/media/upload-request`

Initiates a presigned upload session for secure, UI-friendly file uploads with progress tracking.

**Request Body:**

```json
{
  "filename": "example.jpg",
  "content_type": "image/jpeg",
  "file_size": 1048576
}
```

**Request Fields:**

- `filename` (string, required): Original filename (validated for security)
- `content_type` (string, required): MIME content type (must contain slash)
- `file_size` (integer, required): File size in bytes (max 50MB default)

**Successful Response:**

```json
{
  "media_id": 123,
  "upload_url": "http://localhost:3000/api/v1/media-management/media/upload/upload_abc123?signature=def456&expires=1704067200&size=1048576&type=image%2Fjpeg",
  "upload_token": "upload_abc123",
  "expires_at": "2024-01-01T12:00:00Z",
  "status": "Pending"
}
```

**Security Features:**

- **HMAC-SHA256 signature** for URL tampering protection
- **Expiration timestamps** (15-minute default)
- **File size validation** and limits
- **Content type validation**
- **Dangerous file extension filtering**

**Error Responses:**

**400 Bad Request - File too large:**

```json
{
  "error": "Bad Request",
  "message": "File size 52428800 bytes exceeds maximum allowed size of 50000000 bytes"
}
```

**400 Bad Request - Dangerous extension:**

```json
{
  "error": "Bad Request",
  "message": "File type not allowed: malware.exe"
}
```

**Status Codes:**

- `200 OK` - Upload session created successfully
- `400 Bad Request` - Invalid request (file too large, dangerous extension, invalid content type)

**Example Usage:**

```bash
curl -X POST "http://localhost:3000/api/v1/media-management/media/upload-request" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer <your-jwt-token>" \
  -d '{
    "filename": "photo.jpg",
    "content_type": "image/jpeg",
    "file_size": 2048576
  }'
```

---

### Upload File to Presigned URL

**PUT** `/media/upload/{token}`

Uploads the actual file content using the presigned URL from upload initiation.

**Path Parameters:**

- `token` (string, required): Upload token from initiation response

**Query Parameters (automatically included in presigned URL):**

- `signature` (string, required): HMAC signature for security validation
- `expires` (integer, required): Unix timestamp for URL expiration
- `size` (integer, required): Expected file size in bytes
- `type` (string, required): URL-encoded content type

**Request Body:** Raw file data (binary)

**Successful Response:**

```json
{
  "media_id": 123,
  "content_hash": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "processing_status": "Processing",
  "upload_url": null
}
```

**Error Responses:**

**400 Bad Request - Expired URL:**

```json
{
  "error": "Bad Request",
  "message": "Upload URL has expired at 2024-01-01T11:00:00Z"
}
```

**400 Bad Request - Size mismatch:**

```json
{
  "error": "Bad Request",
  "message": "File size mismatch: expected 1048576 bytes, got 1024000 bytes"
}
```

**401 Unauthorized - Invalid signature:**

```json
{
  "error": "Unauthorized",
  "message": "Invalid upload signature"
}
```

**Status Codes:**

- `200 OK` - File uploaded and processing started
- `400 Bad Request` - Invalid signature, expired URL, or file size mismatch
- `401 Unauthorized` - Invalid or expired signature

**Example Usage:**

```bash
# Use the upload_url from the initiation response
curl -X PUT \
  "http://localhost:3000/api/v1/media-management/media/upload/upload_abc123?\
signature=def456&expires=1704067200&size=1048576&type=image%2Fjpeg" \
  --data-binary @photo.jpg \
  -H "Content-Type: image/jpeg"
```

---

### Get Upload/Processing Status

**GET** `/media/{id}/status`

Retrieves the current status of a media upload, including processing progress and any error information.

**Path Parameters:**

- `id` (integer, required): Media ID from upload initiation

**Successful Response:**

```json
{
  "media_id": 123,
  "status": "Complete",
  "progress": 100,
  "error_message": null,
  "download_url": "http://localhost:3000/api/v1/media-management/media/123/download",
  "processing_time_ms": 2500,
  "uploaded_at": "2024-01-01T12:00:00Z",
  "completed_at": "2024-01-01T12:00:02Z"
}
```

**Status Values:**

- `"Pending"` - Upload session created, file not yet uploaded
- `"Processing"` - File uploaded, currently being processed
- `"Complete"` - Processing finished, file ready for use
- `"Failed"` - Processing failed, see error_message

**Status Codes:**

- `200 OK` - Status retrieved successfully
- `404 Not Found` - Media not found

**Example Usage:**

```bash
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/123/status"
```

---

### List Media

**GET** `/media/`

Retrieve a list of media files with efficient cursor-based pagination and optional filtering.

**Query Parameters:**

- `cursor` (string, optional) - Base64-encoded cursor for pagination navigation
- `limit` (integer, optional) - Maximum number of items to return (default: 50, max: 100, min: 1)
- `status` (string, optional) - Filter by processing status
  - Valid values: `Pending`, `Processing`, `Complete`, `Failed`

**Example Requests:**

```bash
# Get first page (default 50 items)
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/"

# Get first page with custom limit
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/?limit=25"

# Get next page using cursor from previous response
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/?cursor=eyJpZCI6MTI0fQ=="

# Filter by processing status
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/?status=Complete&limit=10"

# Combined filters
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/?cursor=eyJpZCI6MTAwfQ==&limit=20&status=Complete"
```

**Response Format:**

```json
{
  "data": [
    {
      "id": 123,
      "content_hash": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
      "original_filename": "example-image.jpg",
      "media_type": "image/jpeg",
      "media_path": "ab/cd/ef/abcdef123456",
      "file_size": 1048576,
      "processing_status": "Complete",
      "uploaded_at": "2025-01-15T10:30:00Z",
      "updated_at": "2025-01-15T10:30:00Z"
    }
  ],
  "pagination": {
    "next_cursor": "eyJpZCI6MTI0fQ==",
    "prev_cursor": null,
    "page_size": 1,
    "has_next": true,
    "has_prev": false
  }
}
```

**Pagination Fields:**

- `next_cursor`: Base64-encoded cursor for next page (null if last page)
- `prev_cursor`: Reserved for future backward pagination (currently null)
- `page_size`: Number of items in current page
- `has_next`: Boolean indicating if more items available
- `has_prev`: Boolean indicating if previous items exist (based on cursor presence)

**Cursor-Based Pagination Benefits:**

- More efficient than offset-based pagination for large datasets
- Consistent results even when data is modified during pagination
- Scales better with database indexing

**Status Codes:**

- `200 OK` - Successfully retrieved media list
- `400 Bad Request` - Invalid query parameters (e.g., invalid cursor format)

---

### Get Media by ID

**GET** `/media/{id}`

Retrieve detailed information about a specific media file.

**Path Parameters:**

- `id` (integer) - The unique identifier of the media file

**Example Request:**

```bash
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/123"
```

**Successful Response:**

```json
{
  "id": 123,
  "content_hash": "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
  "original_filename": "example.jpg",
  "media_type": "image/jpeg",
  "media_path": "ab/cd/ef/abcdef123456",
  "file_size": 1048576,
  "processing_status": "Complete",
  "uploaded_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z"
}
```

**Error Response:**

```json
{
  "error": "Not Found",
  "message": "Media with ID 123"
}
```

**Processing Status Values:**

- `"Pending"` - Media uploaded but not yet processed
- `"Processing"` - Media currently being processed
- `"Complete"` - Media successfully processed and available
- `"Failed"` - Processing failed

**Status Codes:**

- `200 OK` - Successfully retrieved media metadata
- `400 Bad Request` - Invalid media ID format
- `404 Not Found` - Media not found

---

### Delete Media

**DELETE** `/media/{id}`

Permanently delete a media file and its associated database record. This operation removes both the file from storage
and the metadata from the database.

**Path Parameters:**

- `id` (integer) - The unique identifier of the media file to delete

**Example Request:**

```http
DELETE /media/123
```

**Success Response:**

```http
HTTP/1.1 204 No Content
```

**Error Responses:**

**Media Not Found:**

```json
{
  "error": "Not Found",
  "message": "Media with ID 123"
}
```

**Internal Server Error:**

```json
{
  "error": "Internal Server Error",
  "message": "Failed to delete media file"
}
```

**Status Codes:**

- `204 No Content` - Media successfully deleted
- `404 Not Found` - Media with specified ID not found
- `500 Internal Server Error` - Storage or database operation failed

**Security Considerations:**

- Users can only delete media files they own
- Content-addressable storage prevents path traversal attacks
- Audit logging records all deletion operations
- Operation continues even if storage deletion fails (handles pre-deleted files)

**Storage Behavior:**

- Files are permanently removed from the filesystem
- Content deduplication is respected - files shared between multiple media records are preserved
- Empty directories are cleaned up after file deletion
- Graceful handling of partial failures (e.g., file deleted but database operation fails)

**Example Usage:**

```bash
# Delete media with ID 123
curl -X DELETE \
  -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/123"

# Using Kubernetes service URL
curl -X DELETE \
  -H "Authorization: Bearer <your-jwt-token>" \
  "http://sous-chef-proxy.local/api/v1/media-management/media/123"

# Verify deletion was successful (should return 404)
curl -H "Authorization: Bearer <your-jwt-token>" \
  "http://localhost:3000/api/v1/media-management/media/123"
```

---

### Download Media

**GET** `/media/{id}/download`

Download the actual media file binary data.

**Path Parameters:**

- `id` (integer) - The unique identifier of the media file

**Example Request:**

```bash
GET /media/123/download
```

**Successful Response:**

- **Content-Type**: Based on media type (e.g., `image/jpeg`, `video/mp4`)
- **Content-Length**: Size of the file in bytes
- **Content-Disposition**: `attachment; filename="{original_filename}"`
- **Cache-Control**: `private, max-age=3600` (cached for 1 hour)
- **Body**: Binary file data

**Error Responses:**

**Media Not Found:**

```json
{
  "error": "Not Found",
  "message": "Media with ID 123"
}
```

**Internal Server Error:**

```json
{
  "error": "Internal Server Error",
  "message": "Failed to retrieve media file"
}
```

**Status Codes:**

- `200 OK` - File downloaded successfully
- `400 Bad Request` - Invalid media ID format
- `404 Not Found` - Media not found
- `500 Internal Server Error` - Storage or database error

**Example Usage:**

```bash
# Download media file
curl -H "Authorization: Bearer <your-jwt-token>" \
  -o downloaded_file.jpg \
  "http://localhost:3000/api/v1/media-management/media/123/download"

# Using Kubernetes service URL
curl -H "Authorization: Bearer <your-jwt-token>" \
  -o downloaded_file.jpg \
  "http://sous-chef-proxy.local/api/v1/media-management/media/123/download"
```

---

### Get Media IDs by Recipe

**GET** `/media/recipe/{recipe_id}`

Retrieve media IDs associated with a specific recipe.

**Path Parameters:**

- `recipe_id` (integer) - The unique identifier of the recipe

**Example Request:**

```http
GET /media/recipe/123
```

**Response:**

```json
[1, 2, 3]
```

**Status Codes:**

- `200 OK` - Returns array of media IDs
- `400 Bad Request` - Invalid recipe ID
- `500 Internal Server Error` - Database error

---

### Get Media IDs by Recipe Ingredient

**GET** `/media/recipe/{recipe_id}/ingredient/{ingredient_id}`

Retrieve media IDs associated with a specific ingredient in a recipe.

**Path Parameters:**

- `recipe_id` (integer) - The unique identifier of the recipe
- `ingredient_id` (integer) - The unique identifier of the ingredient

**Example Request:**

```http
GET /media/recipe/123/ingredient/456
```

**Response:**

```json
[4, 5]
```

**Status Codes:**

- `200 OK` - Returns array of media IDs
- `400 Bad Request` - Invalid recipe or ingredient ID
- `500 Internal Server Error` - Database error

---

### Get Media IDs by Recipe Step

**GET** `/media/recipe/{recipe_id}/step/{step_id}`

Retrieve media IDs associated with a specific step in a recipe.

**Path Parameters:**

- `recipe_id` (integer) - The unique identifier of the recipe
- `step_id` (integer) - The unique identifier of the step

**Example Request:**

```http
GET /media/recipe/123/step/789
```

**Response:**

```json
[6, 7, 8]
```

**Status Codes:**

- `200 OK` - Returns array of media IDs
- `400 Bad Request` - Invalid recipe or step ID
- `500 Internal Server Error` - Database error

---

## Data Models

### ProcessingStatus

Current status of media processing:

```json
"Pending"     // Awaiting processing
"Processing"  // Currently being processed
"Complete"    // Ready for use
"Failed"      // Processing failed
```

---

## Error Handling

All endpoints follow a consistent error response format:

```json
{
  "error": "Error Type",
  "message": "Detailed error description"
}
```

### Standard Error Types

- `Not Found` - Requested resource does not exist (404)
- `Bad Request` - Invalid request parameters (400)
- `Internal Server Error` - Unexpected server error (500)

---

## Architecture Notes

- **Clean Architecture**: Domain-driven design with clear layer separation
- **Content-Addressable Storage**: Files organized by SHA-256 hash
- **Multi-format Support**: AVIF (primary), WebP (fallback), JPEG (legacy)
- **Async Processing**: Non-blocking file processing pipeline
- **Kubernetes Ready**: Health checks and graceful shutdown
- **Security First**: Path traversal prevention and content validation

---

## Features

The media management service provides comprehensive media handling capabilities:

- **Media Upload**: Direct upload and presigned upload flows with multipart support
- **Media Management**: List, retrieve, and delete media with cursor-based pagination
- **Content Delivery**: Download media files with proper content-type handling
- **Recipe Integration**: Query media associated with recipes, ingredients, and steps
- **Content Deduplication**: Hash-based storage prevents duplicate files
- **Authentication**: OAuth2 JWT-based access control
- **Monitoring**: Health checks, readiness probes, and Prometheus metrics

---

## Configuration

The service supports two runtime modes:

### Local Development

- Configuration: `.env.local` file + environment variables
- Storage: Relative paths (`./media`)
- Logging: Pretty format for readability

### Production/Kubernetes

- Configuration: Environment variables only
- Storage: Absolute container paths (`/app/media`)
- Logging: JSON format for log aggregation

### Key Environment Variables

```bash
# Server Configuration
MEDIA_SERVICE_SERVER_HOST=0.0.0.0
MEDIA_SERVICE_SERVER_PORT=3000

# Database (when implemented)
POSTGRES_HOST=your-postgres-host
POSTGRES_PORT=5432
POSTGRES_DB=recipe_database
MEDIA_MANAGEMENT_DB_USER=your-db-user
MEDIA_MANAGEMENT_DB_PASSWORD=your-db-password

# Storage
MEDIA_SERVICE_STORAGE_BASE_PATH=/app/media
```

---

Generated for Media Management Service v0.1.0
