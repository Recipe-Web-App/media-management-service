# ADR-003: Filesystem Storage Strategy

## Status

Accepted

## Context

We need to choose a storage strategy for media files (images, videos) that balances performance, cost, scalability, and
complexity. The service will handle potentially large files and needs to support efficient retrieval and processing.

## Decision

We will use direct filesystem storage with content-addressable organization instead of database BLOBs or immediate
cloud storage integration.

## Rationale

### Advantages of Filesystem Storage

- **Performance**: Direct file access is faster than database BLOBs or network storage
- **Cost Efficiency**: No additional storage service costs or data transfer fees
- **Simplicity**: Reduced complexity compared to cloud storage integration
- **Caching**: Operating system and reverse proxy caching work naturally
- **Backup**: Standard filesystem backup tools and strategies apply
- **Development**: Easier local development and testing

### Content-Addressable Organization Benefits

- **Deduplication**: Identical files automatically deduplicated by hash
- **Integrity**: Content hash guarantees file hasn't been corrupted
- **Scalability**: Hash-based directory structure handles millions of files
- **Cache-Friendly**: Predictable URLs enable effective CDN caching
- **Immutability**: Files never change once stored (append-only)

### Alternatives Considered

#### Database BLOBs

- **Pros**: ACID transactions, simple backup
- **Cons**: Poor performance, database bloat, complex streaming

#### Cloud Storage (S3, etc.)

- **Pros**: Infinite scalability, managed service
- **Cons**: Network latency, costs, complexity, vendor lock-in

#### Hybrid Approach

- **Pros**: Best of both worlds
- **Cons**: Increased complexity, synchronization challenges

## Consequences

### Positive

- Excellent read performance for file serving
- Natural integration with reverse proxies and CDNs
- Simple backup and disaster recovery strategies
- Cost-effective for moderate file volumes
- Easy to implement and reason about

### Negative

- Manual scaling considerations for very large datasets
- Need to implement our own redundancy if required
- File system limits on directory size and file count
- Need careful cleanup of orphaned files

## Implementation Details

### Directory Structure

```text
/media/
├── originals/           # Original uploaded files
│   ├── ab/cd/ef/        # First 6 chars of SHA-256 hash
│   │   └── abcdef123... # Full hash filename
├── processed/           # Optimized versions
│   ├── ab/cd/ef/
│   │   ├── abcdef123...webp
│   │   ├── abcdef123...avif
│   │   └── abcdef123...thumb.webp
├── temp/               # Upload staging
└── quarantine/         # Failed uploads
```

### Security Measures

- All file operations within defined base directories
- Path traversal prevention through hash-based paths
- File type validation and virus scanning integration
- Separate read/write permissions

### Future Migration Path

- Abstract storage behind trait interface
- Easy migration to cloud storage if needed
- Hybrid approach possible (filesystem + cloud backup)
