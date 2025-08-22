# Storage Strategy

## Overview

The Media Management Service uses a content-addressable storage (CAS) strategy built on the filesystem, optimized for
performance, security, and efficient organization of media files.

## Content-Addressable Storage (CAS)

### Concept

Files are stored and referenced by their content hash (SHA-256) rather than arbitrary names or paths. This approach
provides several key benefits:

- **Automatic Deduplication**: Identical files share the same hash and storage location
- **Data Integrity**: File corruption is immediately detectable through hash verification
- **Immutable Storage**: Files cannot be modified once stored (append-only model)
- **Security**: Hash-based paths prevent directory traversal attacks
- **Cache Efficiency**: Consistent URLs enable aggressive caching strategies

### Hash Algorithm: SHA-256

- **Security**: Cryptographically secure with negligible collision probability
- **Performance**: Good balance of security and computational efficiency
- **Ecosystem**: Wide tool support and industry standard
- **Future-Proof**: Expected to remain secure for decades

## Directory Structure

### Nested Hash Organization

```text
/media/
├── originals/                    # Original uploaded files
│   ├── ab/cd/ef/                # First 6 chars of SHA-256 hash
│   │   └── abcdef123456...      # Full hash filename (64 chars)
│   ├── ab/cd/f0/
│   │   └── abcdf01234567...
│   └── ...
├── processed/                   # Optimized variants
│   ├── ab/cd/ef/
│   │   ├── abcdef123456...avif  # AVIF variant
│   │   ├── abcdef123456...webp  # WebP variant
│   │   └── abcdef123456...thumb.webp # Thumbnail
│   └── ...
├── temp/                        # Upload staging area
│   ├── upload_session_123/
│   └── upload_session_456/
└── quarantine/                  # Failed/suspicious uploads
    ├── virus_detected/
    └── invalid_format/
```

### Path Construction Logic

```rust
fn build_content_path(base_dir: &Path, hash: &str, variant: Option<&str>) -> PathBuf {
    let prefix1 = &hash[0..2];   // First 2 chars
    let prefix2 = &hash[2..4];   // Next 2 chars
    let prefix3 = &hash[4..6];   // Next 2 chars

    let filename = match variant {
        Some(ext) => format!("{}.{}", hash, ext),
        None => hash.to_string(),
    };

    base_dir.join(prefix1).join(prefix2).join(prefix3).join(filename)
}

// Examples:
// Original: /media/originals/ab/cd/ef/abcdef123456789...
// AVIF:     /media/processed/ab/cd/ef/abcdef123456789...avif
// WebP:     /media/processed/ab/cd/ef/abcdef123456789...webp
// Thumb:    /media/processed/ab/cd/ef/abcdef123456789...thumb.webp
```

### Directory Distribution Benefits

- **Filesystem Efficiency**: Avoids too many files in single directory
- **Balanced Load**: Even distribution across directory tree
- **Parallel Access**: Multiple processes can work on different subtrees
- **Scalability**: Structure handles millions of files efficiently

## File Lifecycle

### Upload Process

1. **Streaming Upload**: Content streamed to temporary file while computing hash
2. **Hash Calculation**: SHA-256 computed incrementally during upload
3. **Validation**: File type and content validation after upload complete
4. **Atomic Move**: File moved from temp to final content-addressed location
5. **Database Record**: Metadata stored in PostgreSQL with hash reference
6. **Cleanup**: Temporary files cleaned up on success or failure

### Variant Generation

1. **Original Preservation**: Original file always preserved for re-processing
2. **Format Analysis**: Detect image/video type and characteristics
3. **Optimization Rules**: Apply format-specific compression and sizing
4. **Quality Verification**: Ensure generated variants meet quality standards
5. **Atomic Creation**: Variants created in temp location then moved to final path
6. **Database Update**: Mark variants as available in metadata

### Cleanup and Maintenance

1. **Orphan Detection**: Find files without database references
2. **Reference Counting**: Track active references before deletion
3. **Temp Cleanup**: Remove incomplete uploads after timeout
4. **Integrity Checks**: Periodic verification of file hashes
5. **Disk Space Management**: Archive or delete old/unused content

## Security Implementation

### Path Security

```rust
pub struct SecurePath {
    base_dir: PathBuf,
    hash: String,
}

impl SecurePath {
    pub fn new(base_dir: &Path, content_hash: &str) -> Result<Self, SecurityError> {
        // Validate hash format
        if content_hash.len() != 64 || !content_hash.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(SecurityError::InvalidHash);
        }

        // Ensure base directory is absolute and exists
        let base_dir = base_dir.canonicalize()
            .map_err(|_| SecurityError::InvalidBasePath)?;

        Ok(SecurePath {
            base_dir,
            hash: content_hash.to_lowercase(),
        })
    }

    pub fn build_path(&self, variant: Option<&str>) -> PathBuf {
        // Construct safe nested path
        build_content_path(&self.base_dir, &self.hash, variant)
    }
}
```

### Access Control

- **Sandboxing**: All file operations restricted to defined base directories
- **Validation**: Path components validated before filesystem access
- **Permissions**: Separate read/write permissions for different operations
- **Audit Trail**: All file access logged with user context

### Content Verification

```rust
pub async fn verify_file_integrity(path: &Path, expected_hash: &str) -> Result<bool, IoError> {
    let mut hasher = Sha256::new();
    let mut file = File::open(path).await?;
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 { break; }
        hasher.update(&buffer[..bytes_read]);
    }

    let computed_hash = format!("{:x}", hasher.finalize());
    Ok(computed_hash == expected_hash)
}
```

## Performance Optimizations

### Caching Strategy

- **Operating System Cache**: Leverage filesystem cache for frequently accessed files
- **Application Cache**: In-memory cache for small files (thumbnails)
- **CDN Integration**: Content-addressable URLs perfect for CDN caching
- **Cache Headers**: Aggressive HTTP caching with content-based ETags

### I/O Optimization

- **Async Operations**: All file I/O uses async/await for concurrency
- **Streaming**: Large files streamed without memory buffering
- **Parallel Processing**: Independent hash prefixes processed in parallel
- **SSD Optimization**: Optimized for SSD storage characteristics

### Storage Tiers

```text
Hot Tier (SSD):
├── processed/     # Frequently accessed optimized variants
└── recent/        # Recently uploaded originals

Warm Tier (Standard):
├── originals/     # All original files
└── archive/       # Older processed variants

Cold Tier (Archive):
└── backup/        # Long-term backup storage
```

## Backup and Disaster Recovery

### Backup Strategy

- **Incremental Sync**: Only new hashes need backup (content-addressable benefit)
- **Geographic Distribution**: Multiple regions for disaster recovery
- **Consistency**: Database metadata must stay synchronized with filesystem
- **Verification**: Regular backup integrity checks using hash verification

### Recovery Procedures

- **Point-in-Time Recovery**: Database backups with filesystem snapshots
- **Partial Recovery**: Individual file recovery using hash lookup
- **Cross-Region Failover**: Automatic failover to backup regions
- **Integrity Restoration**: Re-generate corrupted files from originals when possible

## Monitoring and Metrics

### Storage Metrics

- **Disk Usage**: Total storage consumption by tier
- **File Counts**: Number of files per directory level
- **Deduplication Ratio**: Storage savings from identical file elimination
- **Access Patterns**: Hot/warm/cold file access frequency

### Performance Metrics

- **Upload Throughput**: Files processed per second
- **Storage Latency**: Time to store and retrieve files
- **Hash Computation Time**: SHA-256 calculation performance
- **Variant Generation Time**: Processing pipeline performance

### Health Checks

- **Disk Space**: Available storage capacity monitoring
- **Filesystem Health**: Check for filesystem errors or corruption
- **Orphan Detection**: Files without database references
- **Integrity Verification**: Periodic hash verification results
