# Media Management Service - Postman Collection

This directory contains Postman collection and environment files for comprehensive API testing of the Media Management Service.

## 📁 Files Overview

### Collection Files

- **`Media-Management-Service.postman_collection.json`** - Complete API testing collection with organized endpoints for
  media upload, retrieval, processing, and health monitoring

### Environment Files

- **`Media-Management-Local.postman_environment.json`** - Local development environment (localhost:3000)
- **`Media-Management-Development.postman_environment.json`** - Development environment (sous-chef-proxy.local)
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

### 2. Configure Authentication

1. **Set up Private Environment Files:**

   ```bash
   # Copy the environment files and add '-Private' suffix
   cp Media-Management-Local.postman_environment.json \
      Media-Management-Local-Private.postman_environment.json
   cp Media-Management-Development.postman_environment.json \
      Media-Management-Development-Private.postman_environment.json
   ```

2. **Add Real Credentials:**
   Edit your `-Private` files and replace these placeholder values:
   - `REPLACE_WITH_YOUR_JWT_TOKEN` → Your actual OAuth2 JWT token
   - `REPLACE_WITH_YOUR_TEST_USER_PASSWORD` → Your actual test user password

3. **Import Private Environments:**
   - Import your `-Private.postman_environment.json` files into Postman
   - Use these private environments for actual testing
   - The `-Private` files are automatically gitignored

### 3. Basic Testing

1. **Test Health Endpoints:**
   - Run "Health Check" request (should return status: "healthy")
   - Run "Readiness Check" request (should return status: "ready")
   - Run "Metrics Endpoint" request (should return Prometheus format)

2. **Test Media Upload Flow:**
   - Run "1. Initiate Presigned Upload" (creates upload session)
   - Run "2. Upload File to Presigned URL" (uploads actual file)
   - Run "Get Upload Status" (check processing status)
   - Run "Get Created Media" (retrieve media metadata)

## 📋 Collection Structure

### 1. Health & Monitoring

Complete operational status checks:

- **Health Check** - Service liveness probe endpoint (`GET /health`)
- **Readiness Check** - Service readiness probe endpoint (`GET /ready`)
- **Metrics Endpoint** - Prometheus metrics endpoint (`GET /metrics`)

**Features:**

- Automatic response validation
- Performance timing checks
- Dependency status verification
- Prometheus format validation

### 2. Media Upload

File upload endpoints with comprehensive testing:

- **1. Initiate Presigned Upload** - Creates secure upload session with token
- **2. Upload File to Presigned URL** - Uploads file using presigned URL
- **Direct Upload (Legacy)** - Multipart form-data upload

**Features:**

- Presigned upload flow with security validation
- Multipart form-data configuration
- File type and size validation testing
- Automatic response extraction (media ID, upload token, URLs)
- Error scenario testing

### 3. Media Management

Media CRUD operations and metadata management:

- **List All Media** - Retrieve all media files with pagination
- **List Media with Pagination** - Custom pagination and filtering
- **Get Media by ID** - Individual media metadata
- **Get Created Media** - Retrieve media created in current session
- **Get Upload Status** - Check processing status
- **Download Media File** - Download file content
- **Delete Media** - Remove media files

**Features:**

- Cursor-based pagination testing
- Query parameter validation (limit, status filtering)
- Response structure validation
- Auto-population from previous requests
- File download integrity checks

### 4. Recipe Integration

Recipe-related media endpoints:

- **Get Media by Recipe** - Retrieve all media associated with a recipe
- **Get Media by Recipe Ingredient** - Get media for specific recipe ingredients
- **Get Media by Recipe Step** - Get media for specific cooking steps

**Features:**

- Recipe ecosystem integration testing
- Media association validation
- Response array structure verification

### 5. Error Scenarios

Edge case and error handling testing:

- **Get Non-existent Media** - 404 error handling
- **Upload with Invalid Content Type** - Content validation testing
- **Upload File Too Large** - File size limit testing
- **Invalid Route Test** - Route not found testing

**Features:**

- Error response structure validation
- Status code verification
- Error message testing
- Validation boundary testing

## 🌐 Environment Variables

### Base URLs

- **`mediaManagementServiceBaseUrl`** - API endpoint base URL
- **`mediaManagementServiceMetricsUrl`** - Metrics endpoint base URL

### Authentication

- **`mediaManagementServiceAccessToken`** - OAuth2 JWT token (secret type)

### User Credentials (for authentication testing)

- **`mediaManagementServiceTestUserUsername`** - Test user username
- **`mediaManagementServiceTestUserEmail`** - Test user email
- **`mediaManagementServiceTestUserFullName`** - Test user full name
- **`mediaManagementServiceTestUserPassword`** - Test user password (secret type)

### Test Data Variables

- **`mediaManagementServiceTestMediaId`** - Sample media ID for testing (123)
- **`mediaManagementServiceTestRecipeId`** - Sample recipe ID for testing (456)
- **`mediaManagementServiceTestIngredientId`** - Sample ingredient ID for testing (789)
- **`mediaManagementServiceTestStepId`** - Sample step ID for testing (101)

### Dynamic Variables (Auto-managed)

These variables are automatically set by test scripts:

- **`mediaManagementServiceCreatedMediaId`** - Media ID from upload operations
- **`mediaManagementServiceUploadToken`** - Upload token for presigned flow
- **`mediaManagementServiceUploadUrl`** - Presigned upload URL

### File Configuration

- **`mediaManagementServiceTestImagePath`** - Local path to test image file
- **`mediaManagementServiceTestVideoPath`** - Local path to test video file
- **`mediaManagementServiceMaxFileSize`** - Maximum allowed file size (10MB)
- **`mediaManagementServiceSupportedImageTypes`** - Supported image MIME types
- **`mediaManagementServiceSupportedVideoTypes`** - Supported video MIME types

## 🔧 Advanced Features

### Automatic Response Handling

The collection includes comprehensive test scripts that:

- **Extract Response Data**: Automatically capture media IDs, upload tokens, URLs, and other important data
- **Validate Structure**: Check response format and required fields
- **Status Code Validation**: Verify appropriate HTTP status codes
- **Chain Requests**: Use extracted data in subsequent requests

### File Upload Testing

- **Presigned Upload Flow**: Complete workflow from initiation to file upload
- **Security Validation**: HMAC signature and expiration testing
- **Content-Type Testing**: Verify different media types are handled correctly
- **File Size Validation**: Test file size limits and constraints
- **Error Scenarios**: Invalid file types, missing files, oversized uploads

### Request Chaining

- Initiate upload → Extract token and URL → Upload file → Check status → Get metadata
- Automatic variable population for seamless testing workflows
- Cross-request data persistence

## 📝 Usage Workflows

### Getting Started

1. Start the Media Management Service locally (`cargo run`)
2. Import the collection and local environment
3. Create and import private environment with real credentials
4. Select "Media Management Service - Local-Private" environment
5. Run health checks to verify connectivity

### Testing Upload Flow

1. **Health Check** - Verify service is running
2. **1. Initiate Presigned Upload** - Creates secure upload session
3. **2. Upload File to Presigned URL** - Upload actual file
4. **Get Upload Status** - Check processing progress
5. **Get Created Media** - Retrieve metadata
6. **Download Media File** - Verify file integrity

### Testing Management Operations

1. **List All Media** - Check media catalog
2. **List Media with Pagination** - Test pagination and filtering
3. **Get Media by ID** - Test metadata retrieval
4. **Delete Media** - Test file deletion (be careful with real data)

### Development Testing

1. Use "Media Management Service - Development" environment
2. Update URLs to match your development server
3. Run the same testing workflow as local development
4. Test Recipe Integration endpoints with real recipe data

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
- Use placeholder values in shared environment files

### File Upload Security

- Test with various file types to verify validation
- Check file size limits are enforced
- Verify malicious file rejection
- Test path traversal prevention

### Authentication Testing

- Use real JWT tokens for authenticated endpoints
- Test token expiration handling
- Verify unauthorized access protection
- Test different user permission levels

## 🌍 Environment Switching

**Local Development:**

- API: `http://localhost:3000/api/v1/media-management`
- Metrics: `http://localhost:3000/metrics`
- Health: `http://localhost:3000/api/v1/media-management/health`

**Development Environment:**

- API: `http://sous-chef-proxy.local/api/v1/media-management`
- Metrics: `http://sous-chef-proxy.local/metrics`
- Health: `http://sous-chef-proxy.local/api/v1/media-management/health`

Switch between environments using the environment selector dropdown in Postman's top-right corner.

## 🚧 Current Implementation Status

### Working Endpoints ✅

- Health check endpoints (`/health`, `/ready`)
- Metrics endpoint (`/metrics`)
- Route structure and middleware
- All media management endpoints (upload, list, get, delete)
- Presigned upload flow
- Recipe integration endpoints

### Key Features ✅

- OAuth2 JWT authentication
- Content-addressable storage
- File type and size validation
- Cursor-based pagination
- Comprehensive error handling
- Prometheus metrics integration

## 🐛 Troubleshooting

### Common Issues

1. **Connection Refused**
   - Verify the service is running (`cargo run`)
   - Check the correct port (default: 3000)
   - Ensure environment URL matches service configuration

2. **401 Unauthorized**
   - Verify your JWT token is valid and not expired
   - Ensure the token includes required scopes
   - Check that the token is properly set in environment variables

3. **404 Errors**
   - Verify API base URL includes `/api/v1/media-management`
   - Check endpoint paths match the collection requests
   - Ensure media IDs exist before testing retrieval endpoints

4. **File Upload Issues**
   - Ensure Content-Type is set correctly for uploads
   - Verify file paths in environment variables
   - Check file exists and is accessible
   - Verify file size doesn't exceed limits

### Getting Help

- Check service logs when running `cargo run`
- Verify environment variable configuration
- Test health endpoints first to establish connectivity
- Use Postman Console to debug request/response details
- Check the main API documentation in `../docs/api/API.md`

## 📚 Additional Resources

- **[Service Documentation](../README.md)** - Complete service overview
- **[Development Guide](../CLAUDE.md)** - Development guidance and commands
- **[API Documentation](../docs/api/API.md)** - Detailed API specification
- **[Architecture Docs](../docs/architecture/)** - System design documentation

This collection provides a comprehensive foundation for testing the Media Management Service API with automatic token
management, response validation, and seamless request chaining workflows.
