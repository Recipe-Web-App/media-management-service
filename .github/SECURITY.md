# Security Policy

## Supported Versions

We release security updates for the following versions:

| Version  | Supported          |
| -------- | ------------------ |
| latest   | :white_check_mark: |
| < latest | :x:                |

We recommend always running the latest version for security patches.

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues.**

### Private Reporting (Preferred)

Report security vulnerabilities using [GitHub Security Advisories](https://github.com/Recipe-Web-App/media-management-service/security/advisories/new).

This allows us to:

- Discuss the vulnerability privately
- Develop and test a fix
- Coordinate disclosure timing
- Issue a CVE if necessary

### What to Include

When reporting a vulnerability, please include:

1. **Description** - Clear description of the vulnerability
2. **Impact** - What can an attacker achieve?
3. **Reproduction Steps** - Step-by-step instructions to reproduce
4. **Affected Components** - Which parts of the service are affected
5. **Suggested Fix** - If you have ideas for remediation
6. **Environment** - Version, configuration, deployment details
7. **Proof of Concept** - Code or requests demonstrating the issue (if safe to share)

### Example Report

```text
Title: Path Traversal in File Upload

Description: The file upload endpoint does not properly sanitize file paths...

Impact: An attacker can write files outside the designated storage directory...

Steps to Reproduce:
1. Upload file with name "../../../etc/passwd"
2. File is written outside media directory
3. Server filesystem is compromised

Affected: src/infrastructure/storage/filesystem.rs

Suggested Fix: Sanitize file paths and reject ../ sequences

Environment: v0.1.0, Docker deployment
```

## Response Timeline

- **Initial Response**: Within 48 hours
- **Status Update**: Within 7 days
- **Fix Timeline**: Varies by severity (critical: days, high: weeks, medium: months)

## Severity Levels

### Critical

- Remote code execution
- Path traversal allowing system file access
- SQL injection
- Mass data exposure
- Arbitrary file upload/execution

### High

- File content validation bypass
- Unauthorized access to user files
- Denial of service affecting all users
- Storage quota bypass
- Malware upload without detection

### Medium

- Information disclosure (limited)
- CSRF vulnerabilities
- Rate limiting bypass
- Metadata leakage
- Insufficient file type validation

### Low

- Verbose error messages
- Security header issues
- Best practice violations
- Missing file size limits

## Security Features

This service implements multiple security layers:

### File Upload Security

- **Content Validation** - Magic number verification for file types
- **Size Limits** - Configurable maximum file sizes
- **File Type Restrictions** - Whitelist of allowed media types
- **Path Sanitization** - Prevents path traversal attacks
- **Content-Addressable Storage** - SHA-256 based file organization
- **Virus Scanning Integration** - Hooks for malware detection
- **Metadata Stripping** - Removes potentially sensitive EXIF data

### Application Security

- **Input Validation** - All inputs sanitized and validated
- **SQL Injection Protection** - Compile-time checked queries (SQLx)
- **Rate Limiting** - Per-IP request throttling
- **CORS Protection** - Configurable cross-origin policies
- **Secure Headers** - CSP, HSTS, X-Frame-Options, etc.

### Storage Security

- **File Integrity** - SHA-256 checksums for all files
- **Deduplication** - Content-addressable storage prevents duplicates
- **Access Control** - User-based file ownership
- **Temporary File Cleanup** - Automatic removal of processing artifacts
- **Storage Quotas** - Per-user and system-wide limits

### Infrastructure

- **Secret Management** - Secrets via environment variables (never in code)
- **Audit Logging** - Comprehensive file operation logging
- **Health Monitoring** - Liveness/readiness probes with dependency validation
- **TLS Support** - HTTPS with configurable certificates
- **Database Encryption** - Optional TLS for PostgreSQL connections

## Security Best Practices

### For Operators

1. **Use TLS/HTTPS** - Always encrypt traffic in production
2. **Monitor Logs** - Watch for suspicious upload patterns
3. **Update Dependencies** - Keep Rust crates current via Dependabot
4. **Limit Exposure** - Use network policies and firewalls
5. **Configure File Limits** - Set appropriate size and type restrictions
6. **Enable Virus Scanning** - Integrate malware detection service
7. **Backup Storage** - Regular backups of media files
8. **Database Security** - Use connection encryption and least privilege
9. **Monitor Storage Usage** - Alert on unusual growth patterns
10. **Review Upload Logs** - Check for abuse or attack patterns

### For Developers

1. **Never Commit Secrets** - Use `.env.local` (gitignored)
2. **Validate Inputs** - Sanitize all user inputs and file uploads
3. **Use Parameterized Queries** - SQLx prevents SQL injection
4. **Handle Errors Securely** - Don't leak sensitive info in errors
5. **Run Security Checks** - Use `cargo deny check` before committing
6. **Review Dependencies** - Check for known vulnerabilities
7. **Test File Validation** - Include security test cases for uploads
8. **Follow Clean Architecture** - Maintain security boundaries between layers

## Security Checklist

Before deploying:

- [ ] TLS/HTTPS configured
- [ ] File upload size limits configured
- [ ] Allowed file types whitelist configured
- [ ] Rate limiting enabled
- [ ] CORS whitelist configured
- [ ] Secrets in environment variables (not code)
- [ ] Database encryption at rest and in transit
- [ ] Security headers enabled
- [ ] Audit logging enabled
- [ ] Dependencies updated (`cargo update`)
- [ ] Security scan passed (`cargo deny check`)
- [ ] Network policies applied
- [ ] Monitoring and alerting configured
- [ ] Storage quotas configured
- [ ] Virus scanning integrated (if required)

## Known Security Considerations

### File Storage

- Files stored with SHA-256 hash-based paths
- Content deduplication prevents storage exhaustion
- Temporary files cleaned up automatically
- Orphaned files detected and removed

### Database Security

- PostgreSQL connections use connection pooling (SQLx)
- Credentials via environment variables
- Optional TLS for database connections
- Compile-time checked queries prevent SQL injection

### Media Processing

- Image processing uses safe Rust libraries (image-rs)
- Video processing via ez-ffmpeg wrapper
- Processing timeouts prevent resource exhaustion
- Sandboxed processing environment recommended

### API Security

- Cursor-based pagination prevents offset attacks
- JWT-based authentication (OAuth2 service integration)
- Per-user file ownership enforcement
- Presigned URLs with HMAC signatures and expiration

## Disclosure Policy

We follow **coordinated disclosure**:

1. Vulnerability reported privately
2. We confirm and develop fix
3. Fix tested and released
4. Public disclosure after fix is deployed
5. Credit given to reporter (if desired)

## Security Updates

Subscribe to:

- [GitHub Security Advisories](https://github.com/Recipe-Web-App/media-management-service/security/advisories)
- [Release Notes](https://github.com/Recipe-Web-App/media-management-service/releases)
- Watch repository for security patches

## Contact

For security concerns: Use [GitHub Security Advisories](https://github.com/Recipe-Web-App/media-management-service/security/advisories/new)

For general questions: See [SUPPORT.md](SUPPORT.md)

## Acknowledgments

We thank security researchers who responsibly disclose vulnerabilities. Contributors will be acknowledged (with
permission) in:

- Security advisories
- Release notes
- This document

Thank you for helping keep this project secure!
