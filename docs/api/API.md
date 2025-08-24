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

Kubernetes liveness probe endpoint.

**Response:**

```json
{
  "status": "healthy",
  "timestamp": "2024-08-24T10:30:00.123Z",
  "service": "media-management-service"
}
```

**Status Codes:**

- `200 OK` - Service is healthy

---

### Readiness Check

**GET** `/ready`

Kubernetes readiness probe endpoint.

**Response:**

```json
{
  "status": "ready",
  "timestamp": "2024-08-24T10:30:00.123Z",
  "checks": {
    "database": "not_configured",
    "storage": "ok"
  }
}
```

**Status Codes:**

- `200 OK` - Service is ready to accept requests

---

## Media Endpoints

### Upload Media

**POST** `/media/`

Upload a new media file to the system.

**Status**: 🚧 Not Implemented

**Request Headers:**

- `Content-Type: multipart/form-data` (planned)

**Request Body:** (planned)

```json
{
  "filename": "example.jpg"
}
```

**Response:**

```json
{
  "error": "Not Implemented",
  "message": "Media upload functionality is not yet implemented"
}
```

**Status Codes:**

- `501 Not Implemented` - Endpoint not yet implemented

**Planned Response** (when implemented):

```json
{
  "media_id": "550e8400-e29b-41d4-a716-446655440000",
  "content_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  "processing_status": "Pending",
  "upload_url": "https://example.com/media/550e8400-e29b-41d4-a716-446655440000"
}
```

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
    "id": "550e8400-e29b-41d4-a716-446655440000",
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

- `id` (UUID) - The unique identifier of the media file

**Example Request:**

```http
GET /media/550e8400-e29b-41d4-a716-446655440000
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
  "id": "550e8400-e29b-41d4-a716-446655440000",
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

### Download Media

**GET** `/media/{id}/download`

Download the actual media file binary data.

**Status**: 🚧 Not Implemented

**Path Parameters:**

- `id` (UUID) - The unique identifier of the media file

**Example Request:**

```http
GET /media/550e8400-e29b-41d4-a716-446655440000/download
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
