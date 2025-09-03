# Security Model

## Overview

The Media Management Service implements a comprehensive security model designed to protect against common
vulnerabilities while maintaining performance and usability. Security is implemented at multiple layers with
defense-in-depth principles.

## Threat Model

### Identified Threats

1. **Malicious File Uploads**: Virus, malware, or executable content disguised as media
2. **Path Traversal Attacks**: Attempts to access files outside designated areas
3. **Content Injection**: Malicious content embedded in valid file formats
4. **Resource Exhaustion**: Large files or excessive requests causing DoS
5. **Data Integrity**: File corruption or tampering during storage/transfer
6. **Unauthorized Access**: Access to files without proper permissions
7. **Information Disclosure**: Sensitive metadata or content exposure

### Security Boundaries

- **Network Perimeter**: TLS termination and firewall protection
- **Application Layer**: Input validation and business logic security
- **Storage Layer**: Filesystem sandboxing and access controls
- **Data Layer**: Database security and backup protection

## Input Validation and Sanitization

### File Upload Validation

```rust
pub struct FileValidator {
    allowed_types: Vec<MimeType>,
    max_size: u64,
    magic_byte_checker: MagicByteChecker,
}

impl FileValidator {
    pub async fn validate_upload(&self, upload: &FileUpload) -> Result<ValidationResult, SecurityError> {
        // 1. Size validation
        if upload.size > self.max_size {
            return Err(SecurityError::FileTooLarge);
        }

        // 2. MIME type validation (header check)
        if !self.allowed_types.contains(&upload.declared_mime_type) {
            return Err(SecurityError::UnsupportedFileType);
        }

        // 3. Magic byte validation (actual content check)
        let actual_type = self.magic_byte_checker.detect_type(&upload.content)?;
        if actual_type != upload.declared_mime_type {
            return Err(SecurityError::MimeTypeMismatch);
        }

        // 4. Content analysis for embedded threats
        self.scan_for_threats(&upload.content).await?;

        Ok(ValidationResult::Valid)
    }
}
```

### Content-Type Verification

- **Magic Byte Detection**: Verify actual file type matches declared MIME type
- **Header Analysis**: Parse file headers to detect format inconsistencies
- **Embedded Content**: Scan for embedded executables or scripts
- **Metadata Stripping**: Remove potentially dangerous metadata from images

### Input Sanitization

```rust
pub fn sanitize_filename(filename: &str) -> Result<String, SecurityError> {
    // Remove path separators and control characters
    let sanitized = filename
        .chars()
        .filter(|c| !c.is_control() && *c != '/' && *c != '\\' && *c != '..')
        .collect::<String>();

    // Limit length and ensure not empty
    if sanitized.is_empty() || sanitized.len() > 255 {
        return Err(SecurityError::InvalidFilename);
    }

    // Prevent reserved names
    if RESERVED_NAMES.contains(&sanitized.to_lowercase().as_str()) {
        return Err(SecurityError::ReservedFilename);
    }

    Ok(sanitized)
}
```

## Path Security and Sandboxing

### Content-Addressable Security

The content-addressable storage strategy inherently prevents path traversal attacks:

```rust
pub struct SecurePathBuilder {
    base_directories: HashMap<StorageType, PathBuf>,
}

impl SecurePathBuilder {
    pub fn build_safe_path(&self, storage_type: StorageType, content_hash: &str) -> Result<PathBuf, SecurityError> {
        // Validate hash format (must be valid SHA-256)
        if !is_valid_sha256_hash(content_hash) {
            return Err(SecurityError::InvalidContentHash);
        }

        // Get base directory for storage type
        let base_dir = self.base_directories.get(&storage_type)
            .ok_or(SecurityError::InvalidStorageType)?;

        // Build safe nested path using hash prefix
        let safe_path = base_dir
            .join(&content_hash[0..2])
            .join(&content_hash[2..4])
            .join(&content_hash[4..6])
            .join(content_hash);

        // Verify the constructed path is within base directory
        if !safe_path.starts_with(base_dir) {
            return Err(SecurityError::PathTraversalAttempt);
        }

        Ok(safe_path)
    }
}
```

### Filesystem Sandboxing

- **Chroot-like Isolation**: All operations restricted to defined base directories
- **Path Canonicalization**: Resolve symlinks and relative paths before validation
- **Permission Isolation**: Separate permissions for read, write, and delete operations
- **Directory Traversal Prevention**: Hash-based paths eliminate traversal possibilities

## Authentication and Authorization

### OAuth2 Integration

The service implements comprehensive OAuth2 authentication with support for both user authentication and
service-to-service communication:

```rust
pub struct OAuth2AuthMiddleware {
    oauth2_client: Arc<OAuth2Client>,
    jwt_service: Arc<JwtService>,
    config: OAuth2Config,
}

#[async_trait]
impl<S> Middleware<S> for OAuth2AuthMiddleware {
    async fn call(&self, req: Request<Body>, next: Next<S>) -> Result<Response<Body>, Error> {
        // Extract JWT from Authorization header
        let token = extract_bearer_token(&req)
            .ok_or(AuthError::MissingToken)?;

        // Choose validation strategy based on configuration
        let claims = if self.config.introspection_enabled {
            // Online validation via OAuth2 service
            let token_info = self.oauth2_client
                .introspect_token(&token)
                .await
                .map_err(|_| AuthError::InvalidToken)?;

            if !token_info.active {
                return Err(AuthError::InvalidToken);
            }

            Claims::from_introspection(&token_info)
        } else {
            // Offline JWT validation
            self.jwt_service.validate_token(&token)
                .map_err(|_| AuthError::InvalidToken)?
        };

        // Check required scopes
        if !self.has_required_scopes(&claims.scopes) {
            return Err(AuthError::InsufficientPermissions);
        }

        // Add user context to request
        let mut req = req;
        req.extensions_mut().insert(UserContext::from(claims));

        next.run(req).await
    }
}
```

### JWT Claims Structure

OAuth2 JWT tokens use a standardized claims format:

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub iss: String,              // Issuer (OAuth2 service)
    pub aud: Vec<String>,         // Audience (service identifiers)
    pub sub: String,              // Subject (user ID)
    pub client_id: String,        // OAuth2 client ID
    pub user_id: Option<String>,  // User ID (for user tokens)
    pub scopes: Vec<String>,      // OAuth2 scopes
    #[serde(rename = "type")]
    pub token_type: String,       // Token type (user/client_credentials)
    pub exp: usize,               // Expiration time
    pub iat: usize,               // Issued at
    pub nbf: usize,               // Not before
    pub jti: String,              // JWT ID
}
```

### Authentication Strategies

#### 1. JWT Validation (Offline)

Fast, local validation using shared secret:

- **Advantages**: Low latency, no network dependency
- **Use Case**: High-performance scenarios, internal services
- **Configuration**: `OAUTH2_INTROSPECTION_ENABLED=false`

#### 2. Token Introspection (Online)

Authoritative validation via OAuth2 service API:

- **Advantages**: Real-time token status, immediate revocation support
- **Use Case**: Security-critical operations, token revocation scenarios
- **Configuration**: `OAUTH2_INTROSPECTION_ENABLED=true`

### Service-to-Service Authentication

OAuth2 Client Credentials Flow for microservice authentication:

```rust
pub struct ServiceAuthenticator {
    oauth2_client: Arc<OAuth2Client>,
    required_scopes: Vec<String>,
}

impl ServiceAuthenticator {
    pub async fn get_service_token(&self) -> Result<String, AuthError> {
        let token = self.oauth2_client
            .get_client_credentials_token(&self.required_scopes)
            .await?;

        Ok(token.access_token)
    }

    pub async fn authenticated_request(&self, url: &str) -> Result<Response, Error> {
        let token = self.get_service_token().await?;

        let response = reqwest::Client::new()
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await?;

        Ok(response)
    }
}
```

### Permission Model

```rust
pub enum FilePermission {
    Read,
    Write,
    Delete,
    Process,
}

pub struct AccessControl {
    user_permissions: HashMap<UserId, Vec<FilePermission>>,
    file_ownership: HashMap<ContentHash, UserId>,
}

impl AccessControl {
    pub fn check_permission(&self, user_id: &UserId, file_hash: &ContentHash, permission: FilePermission) -> bool {
        // Check if user has global permission
        if let Some(permissions) = self.user_permissions.get(user_id) {
            if permissions.contains(&permission) {
                return true;
            }
        }

        // Check file ownership
        if let Some(owner) = self.file_ownership.get(file_hash) {
            return owner == user_id;
        }

        false
    }
}
```

## Content Security and Integrity

### Hash-Based Integrity

```rust
pub async fn verify_content_integrity(file_path: &Path, expected_hash: &str) -> Result<IntegrityResult, SecurityError> {
    let mut hasher = Sha256::new();
    let mut file = File::open(file_path).await
        .map_err(|_| SecurityError::FileAccessError)?;

    let mut buffer = [0; 8192];
    loop {
        let bytes_read = file.read(&mut buffer).await
            .map_err(|_| SecurityError::ReadError)?;
        if bytes_read == 0 { break; }
        hasher.update(&buffer[..bytes_read]);
    }

    let computed_hash = format!("{:x}", hasher.finalize());

    if computed_hash == expected_hash {
        Ok(IntegrityResult::Valid)
    } else {
        Ok(IntegrityResult::Corrupted {
            expected: expected_hash.to_string(),
            actual: computed_hash
        })
    }
}
```

### Malware Scanning Integration

```rust
pub trait MalwareScanner {
    async fn scan_file(&self, file_path: &Path) -> Result<ScanResult, ScanError>;
}

pub struct ClamAvScanner {
    socket_path: PathBuf,
    timeout: Duration,
}

impl MalwareScanner for ClamAvScanner {
    async fn scan_file(&self, file_path: &Path) -> Result<ScanResult, ScanError> {
        // Integration with ClamAV daemon
        let mut stream = UnixStream::connect(&self.socket_path).await?;

        // Send scan command
        stream.write_all(b"zSCAN ").await?;
        stream.write_all(file_path.as_os_str().as_bytes()).await?;
        stream.write_all(b"\0").await?;

        // Read response
        let mut response = String::new();
        stream.read_to_string(&mut response).await?;

        match response.trim() {
            response if response.ends_with("OK") => Ok(ScanResult::Clean),
            response if response.contains("FOUND") => Ok(ScanResult::Infected),
            _ => Ok(ScanResult::Error),
        }
    }
}
```

## Network Security

### TLS Configuration

```rust
pub fn create_tls_config() -> Result<TlsConfig, TlsError> {
    TlsConfig::builder()
        .with_cipher_suites(&[
            CipherSuite::TLS13_AES_256_GCM_SHA384,
            CipherSuite::TLS13_CHACHA20_POLY1305_SHA256,
        ])
        .with_protocol_versions(&[ProtocolVersion::TLSv1_3])
        .with_certificate_chain(load_certificate_chain()?)
        .with_private_key(load_private_key()?)
        .build()
}
```

### CORS Configuration

```rust
pub fn create_cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origins(AllowOrigin::list([
            "https://recipes.example.com".parse().unwrap(),
        ]))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
        .max_age(Duration::from_secs(3600))
}
```

### Rate Limiting

```rust
pub struct RateLimiter {
    redis_client: RedisClient,
    limits: HashMap<EndpointPattern, RateLimit>,
}

impl RateLimiter {
    pub async fn check_rate_limit(&self, user_id: &UserId, endpoint: &str) -> Result<RateLimitResult, RateLimitError> {
        let key = format!("rate_limit:{}:{}", user_id, endpoint);
        let current_count = self.redis_client.incr(&key, 1).await?;

        if current_count == 1 {
            // Set expiration on first request
            self.redis_client.expire(&key, 3600).await?;
        }

        let limit = self.get_limit_for_endpoint(endpoint);

        if current_count > limit.max_requests {
            Ok(RateLimitResult::Exceeded)
        } else {
            Ok(RateLimitResult::Allowed {
                remaining: limit.max_requests - current_count
            })
        }
    }
}
```

## Audit and Monitoring

### Security Event Logging

```rust
pub struct SecurityLogger {
    logger: Logger,
}

impl SecurityLogger {
    pub fn log_security_event(&self, event: SecurityEvent) {
        let log_entry = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "event_type": event.event_type,
            "severity": event.severity,
            "user_id": event.user_id,
            "ip_address": event.ip_address,
            "user_agent": event.user_agent,
            "details": event.details,
            "correlation_id": event.correlation_id,
        });

        match event.severity {
            Severity::Critical => self.logger.error("{}", log_entry),
            Severity::High => self.logger.warn("{}", log_entry),
            Severity::Medium => self.logger.info("{}", log_entry),
            Severity::Low => self.logger.debug("{}", log_entry),
        }
    }
}
```

### Intrusion Detection

- **Failed Authentication Attempts**: Monitor and alert on suspicious login patterns
- **Path Traversal Attempts**: Log and block requests with traversal patterns
- **File Upload Anomalies**: Detect unusual file types or sizes
- **Rate Limit Violations**: Track and respond to abuse patterns

## Security Configuration

### Environment-Based Security Settings

```rust
pub struct SecurityConfig {
    pub max_file_size: u64,
    pub allowed_mime_types: Vec<String>,
    pub enable_malware_scanning: bool,
    pub jwt_secret: String,
    pub rate_limit_requests_per_hour: u32,
    pub enable_audit_logging: bool,
}

impl SecurityConfig {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(SecurityConfig {
            max_file_size: env::var("MAX_FILE_SIZE")?.parse()?,
            allowed_mime_types: env::var("ALLOWED_MIME_TYPES")?
                .split(',')
                .map(|s| s.trim().to_string())
                .collect(),
            enable_malware_scanning: env::var("ENABLE_MALWARE_SCANNING")?.parse()?,
            jwt_secret: env::var("JWT_SECRET")?,
            rate_limit_requests_per_hour: env::var("RATE_LIMIT_RPH")?.parse()?,
            enable_audit_logging: env::var("ENABLE_AUDIT_LOGGING")?.parse()?,
        })
    }
}
```

## Incident Response

### Automated Response Procedures

1. **Malware Detection**: Immediate quarantine of infected files
2. **Path Traversal Attempts**: Automatic IP blocking and admin notification
3. **Rate Limit Violations**: Progressive throttling and temporary blocks
4. **Authentication Failures**: Account lockout after threshold exceeded

### Manual Response Procedures

1. **Security Event Investigation**: Detailed analysis of security logs
2. **File Quarantine Review**: Manual review of quarantined content
3. **Access Revocation**: Emergency procedures for compromised accounts
4. **System Hardening**: Post-incident security improvements
