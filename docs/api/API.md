# Media Management Service API Documentation

## Base URL

- **Local Development**: `http://localhost:3000/api/v1/media-management`
- **Kubernetes**: `http://media-management.local/api/v1/media-management`

## Overview

The Media Management Service is a production-ready microservice for handling media file uploads, processing, storage, and
retrieval within the Recipe Web Application ecosystem. Built with Rust using Axum framework and following
Clean/Hexagonal Architecture principles.

## Current Implementation Status

⚠️ **Note**: This service is currently in development. Most media endpoints return placeholder responses and are
not yet fully implemented.

## Authentication

Currently, the service runs without authentication enabled. All endpoints are publicly accessible for development purposes.

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

## Media Endpoints

### Upload Media

**POST** `/media/`

Upload a new media file to the system with automatic content-addressable storage and deduplication.

**Status**: ✅ Implemented

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

_Note: Currently all uploads immediately receive `"Pending"` status. Async processing pipeline is planned for future releases._

---

### List Media

**GET** `/media/`

Retrieve a list of media files with optional filtering and pagination.

**Status**: 🚧 Partially Implemented (returns empty list)

**Query Parameters:**

- `limit` (integer, optional) - Maximum number of items to return
- `offset` (integer, optional) - Number of items to skip for pagination
- `status` (string, optional) - Filter by processing status
  - Valid values: `Pending`, `Processing`, `Complete`, `{"Failed": "error message"}`

**Example Request:**

```http
GET /media/?limit=10&offset=0&status=Complete
```

**Response:**

```json
[]
```

**Status Codes:**

- `200 OK` - Returns empty array (current implementation)

**Planned Response Format** (when implemented):

```json
[
  {
    "id": 123,
    "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "original_filename": "example.jpg",
    "media_type": {
      "Image": {
        "format": "Jpeg",
        "width": 1920,
        "height": 1080
      }
    },
    "file_size": 1048576,
    "processing_status": "Complete",
    "uploaded_at": "2024-08-24T10:30:00Z",
    "updated_at": "2024-08-24T10:35:00Z"
  }
]
```

---

### Get Media by ID

**GET** `/media/{id}`

Retrieve detailed information about a specific media file.

**Status**: 🚧 Not Implemented

**Path Parameters:**

- `id` (integer) - The unique identifier of the media file

**Example Request:**

```http
GET /media/123
```

**Response:**

```json
{
  "error": "Not Found",
  "message": "Media not found"
}
```

**Status Codes:**

- `404 Not Found` - Media not found (current implementation)

**Planned Response Format** (when implemented):

```json
{
  "id": 123,
  "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "original_filename": "example.jpg",
  "media_type": {
    "Video": {
      "format": "Mp4",
      "width": 1280,
      "height": 720,
      "duration_seconds": 120
    }
  },
  "file_size": 5242880,
  "processing_status": "Complete",
  "uploaded_at": "2024-08-24T10:30:00Z",
  "updated_at": "2024-08-24T10:35:00Z"
}
```

---

### Delete Media

**DELETE** `/media/{id}`

Permanently delete a media file and its associated database record. This operation removes both the file from storage
and the metadata from the database.

**Status**: ✅ Implemented

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

- Users can only delete media files they own (when authentication is implemented)
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
curl -X DELETE "http://localhost:3000/api/v1/media-management/media/123"

# Using Kubernetes service URL
curl -X DELETE "http://media-management.local/api/v1/media-management/media/123"

# Verify deletion was successful (should return 404)
curl "http://localhost:3000/api/v1/media-management/media/123"
```

---

### Download Media

**GET** `/media/{id}/download`

Download the actual media file binary data.

**Status**: 🚧 Not Implemented

**Path Parameters:**

- `id` (integer) - The unique identifier of the media file

**Example Request:**

```http
GET /media/123/download
```

**Response:**

```json
{
  "error": "Not Implemented",
  "message": "Media download functionality is not yet implemented"
}
```

**Status Codes:**

- `501 Not Implemented` - Endpoint not yet implemented

**Planned Response** (when implemented):

- Content-Type: Based on media type (e.g., `image/jpeg`, `video/mp4`)
- Body: Binary file data

---

### Get Media IDs by Recipe

**GET** `/media/recipe/{recipe_id}`

Retrieve media IDs associated with a specific recipe.

**Status**: ✅ Implemented

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

**Status**: ✅ Implemented

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

**Status**: ✅ Implemented

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

### MediaType

Media type information with format-specific metadata:

**Image:**

```json
{
  "Image": {
    "format": "Jpeg" | "Png" | "WebP" | "Avif" | "Gif",
    "width": 1920,
    "height": 1080
  }
}
```

**Video:**

```json
{
  "Video": {
    "format": "Mp4" | "Webm" | "Mov" | "Avi",
    "width": 1280,
    "height": 720,
    "duration_seconds": 120
  }
}
```

**Audio:**

```json
{
  "Audio": {
    "format": "Mp3" | "Wav" | "Flac" | "Ogg",
    "duration_seconds": 240,
    "bitrate": 128000
  }
}
```

### ProcessingStatus

Current status of media processing:

```json
"Pending"           // Awaiting processing
"Processing"        // Currently being processed
"Complete"          // Ready for use
{"Failed": "error"} // Processing failed with error message
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

- `Not Implemented` - Feature not yet available (501)
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

## Development Status

### ✅ Completed

- Project structure and architecture
- HTTP server setup with middleware
- Health and readiness endpoints
- Data models and DTOs
- Basic routing structure
- Comprehensive test coverage
- Recipe-related media query endpoints
  - Get media IDs by recipe
  - Get media IDs by recipe ingredient
  - Get media IDs by recipe step

### 🚧 In Progress

- Media upload handling
- File processing pipeline
- Database integration
- Storage backend implementation

### 📋 Planned

- Authentication and authorization
- Rate limiting
- Metrics and monitoring
- Image/video processing
- Multiple storage backends (S3, etc.)
- Content delivery optimization

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
