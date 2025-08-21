# Media Management Service - Postman Collection

This directory contains Postman collection and environment files for comprehensive API testing of the Media Management Service.

## 📁 Files Overview

### Collection Files

- **`Media-Management-Service.postman_collection.json`** - Complete API testing collection with organized endpoints for
  media upload, retrieval, processing, and health monitoring

### Environment Files

- **`Media-Management-Local.postman_environment.json`** - Local development environment (localhost:3000)
- **`Media-Management-Development.postman_environment.json`** - Development environment (media-management.local)
- **`*-Private.postman_environment.json`** - Local-only files with real credentials (gitignored)

## 🚀 Quick Start

### 1. Import Collections and Environments

1. **Import Collection:**
   - Open Postman
   - Click "Import" button
   - Select `Media-Management-Service.postman_collection.json`
   - Collection will appear in your workspace

2. **Import Environment:**
   - Import either environment file:
     - `Media-Management-Local.postman_environment.json` (for local testing)
     - `Media-Management-Development.postman_environment.json` (for dev server)

3. **Select Environment:**
   - Choose the appropriate environment from the dropdown in Postman's top-right corner

### 2. Basic Testing

1. **Test Health Endpoints:**
   - Run "Health Check" request (should return status: "healthy")
   - Run "Readiness Check" request (should return status: "ready")

2. **Test API Endpoints:**
   - All media endpoints currently return "Not Implemented" (501 status)
   - Use these to verify routing is working correctly

## 📋 Collection Structure

### 1. Health & Monitoring

Complete operational status checks:

- **Health Check** - Service liveness probe endpoint (`GET /health`)
- **Readiness Check** - Service readiness probe endpoint (`GET /ready`)

**Features:**

- Automatic response validation
- Performance timing checks
- Dependency status verification

### 2. Media Upload

File upload endpoints with comprehensive testing:

- **Upload Media File** - Generic file upload with multipart form-data
- **Upload Image (JPEG)** - Specific image upload testing
- **Upload Video (MP4)** - Video file upload testing

**Features:**

- Multipart form-data configuration
- File type validation testing
- Automatic response extraction (media ID, content hash)
- Error scenario testing

### 3. Media Retrieval

Media listing and metadata endpoints:

- **List All Media** - Retrieve all media files
- **List Media with Query Parameters** - Pagination and filtering
- **Get Media by ID** - Individual media metadata
- **Get Created Media** - Retrieve media created in current session

**Features:**

- Query parameter testing (limit, offset, status)
- Response structure validation
- Auto-population from previous requests

### 4. Media Download

File download and streaming endpoints:

- **Download Media by ID** - Download file content
- **Download Created Media** - Download files from current session

**Features:**

- Content-Type validation
- Header verification (Content-Disposition, Cache-Control)
- File integrity checks

### 5. Error Scenarios

Edge case and error handling testing:

- **Get Non-existent Media** - 404 error handling
- **Invalid Route** - Route not found testing

**Features:**

- Error response structure validation
- Status code verification
- Error message testing

## 🌐 Environment Variables

### Base URLs

- **`mediaServiceBaseUrl`** - Base service URL
- **`mediaServiceApiUrl`** - API endpoint base URL

### Authentication (Future)

- **`mediaServiceAccessToken`** - Authentication token (auto-managed)
- **`mediaServiceRefreshToken`** - Token refresh (auto-managed)
- **`mediaServiceUserId`** - Current user ID (auto-extracted)

### Test Data

- **`testMediaId`** - Sample media ID for testing
- **`testFilename`** - Sample filename for uploads
- **`createdMediaId`** - Auto-extracted from upload responses
- **`createdContentHash`** - Auto-extracted content hash

### File Paths

- **`testImagePath`** - Local path to test image file
- **`testVideoPath`** - Local path to test video file

### Configuration

- **`maxFileSize`** - Maximum allowed file size (500MB)
- **`supportedImageTypes`** - Supported image MIME types
- **`supportedVideoTypes`** - Supported video MIME types

## 🔧 Advanced Features

### Automatic Response Handling

The collection includes comprehensive test scripts that:

- **Extract Response Data**: Automatically capture media IDs, content hashes, and other important data
- **Validate Structure**: Check response format and required fields
- **Status Code Validation**: Verify appropriate HTTP status codes
- **Chain Requests**: Use extracted data in subsequent requests

### File Upload Testing

- **Multipart Form Support**: Proper configuration for file uploads
- **Content-Type Testing**: Verify different media types are handled correctly
- **File Size Validation**: Test file size limits and constraints
- **Error Scenarios**: Invalid file types, missing files, oversized uploads

### Request Chaining

- Upload a file → Extract media ID → Get metadata → Download file
- Automatic variable population for seamless testing workflows

## 📝 Usage Workflows

### Getting Started

1. Start the Media Management Service locally (`cargo run`)
2. Import the collection and local environment
3. Select "Media Management Service - Local" environment
4. Run health checks to verify connectivity

### Testing File Operations

1. **Health Check** - Verify service is running
2. **Upload Media** - Test file upload (currently returns 501)
3. **List Media** - Check media listing (currently returns empty array)
4. **Get Media** - Test metadata retrieval (currently returns 404)
5. **Download Media** - Test file download (currently returns 501)

### Development Testing

1. Use "Media Management Service - Development" environment
2. Update `mediaServiceBaseUrl` to match your development server
3. Run the same testing workflow as local development

## 🛠️ Customization

### Adding New Endpoints

1. Create new request in appropriate folder
2. Add comprehensive test scripts for validation
3. Use environment variables for URLs and test data
4. Document any new environment variables needed

### Environment Setup

1. Copy environment file for customization
2. Update URLs and credentials as needed
3. Add `-Private` suffix to filename to exclude from Git
4. Import private environment into Postman

### Test Script Enhancement

- Add more comprehensive response validation
- Implement additional error scenario testing
- Enhance automatic data extraction
- Add performance benchmarking

## 🔐 Security Considerations

### Credential Management

- Use `-Private` environment files for real credentials
- Mark sensitive variables as "secret" type in Postman
- Never commit actual passwords or tokens to version control

### File Upload Security

- Test with various file types to verify validation
- Check file size limits are enforced
- Verify malicious file rejection
- Test path traversal prevention

## 🚧 Current Status

### Working Endpoints ✅

- Health check endpoints (`/health`, `/ready`)
- Route structure and middleware

### Placeholder Endpoints ⚠️

- Media upload (returns 501 Not Implemented)
- Media listing (returns empty array)
- Media retrieval (returns 404 Not Found)
- Media download (returns 501 Not Implemented)

### Future Updates 🔮

- Update test scripts when endpoints are implemented
- Add authentication flow when auth is integrated
- Enhance file upload testing with real files
- Add media processing status tracking

## 📚 Additional Resources

- **[Service Documentation](../README.md)** - Complete service overview
- **[API Documentation](../CLAUDE.md)** - Development guidance
- **[Architecture Docs](../docs/architecture/)** - System design documentation

## 🐛 Troubleshooting

### Common Issues

1. **Connection Refused**
   - Verify the service is running (`cargo run`)
   - Check the correct port (default: 3000)
   - Ensure environment URL matches service configuration

2. **404 Errors**
   - Verify API base URL includes `/api/v1/media-management`
   - Check endpoint paths match the collection requests

3. **File Upload Issues**
   - Ensure Content-Type is set to `multipart/form-data`
   - Verify file paths in environment variables
   - Check file exists and is accessible

### Getting Help

- Check service logs when running `cargo run`
- Verify environment variable configuration
- Test health endpoints first to establish connectivity
- Use Postman Console to debug request/response details

This collection provides a comprehensive foundation for testing the Media Management Service API and will evolve as the
service implementation progresses.
