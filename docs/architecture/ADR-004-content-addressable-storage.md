# ADR-004: Content-Addressable Storage Implementation

## Status

Accepted

## Context

Having chosen filesystem storage, we need to decide on the file organization strategy. We need to handle deduplication,
ensure data integrity, and support efficient file retrieval while preventing path traversal attacks.

## Decision

We will implement content-addressable storage (CAS) using SHA-256 hashes for file organization and naming.

## Rationale

### Content-Addressable Storage Benefits

- **Automatic Deduplication**: Identical files share the same hash and storage location
- **Data Integrity**: File corruption is immediately detectable through hash verification
- **Immutability**: Files cannot be modified once stored (append-only model)
- **Security**: Hash-based paths prevent directory traversal attacks
- **Cache Efficiency**: Consistent URLs enable aggressive caching strategies
- **Distributed Friendly**: Easy to replicate and synchronize across systems

### Hash Algorithm Choice: SHA-256

- **Security**: Cryptographically secure, collision-resistant
- **Performance**: Good balance of security and computational efficiency
- **Standard**: Widely supported and well-understood
- **Future-Proof**: Likely to remain secure for the foreseeable future

### Directory Structure Strategy

Using nested directories based on hash prefixes to avoid filesystem limitations:

- **2-2-2 Structure**: `ab/cd/ef/abcdef123...`
- **Balanced Distribution**: Even distribution across directories
- **Filesystem Limits**: Avoids issues with too many files in single directory
- **Efficient Traversal**: Predictable structure for cleanup and verification

## Alternatives Considered

### UUID-Based Naming

- **Pros**: Simple generation, guaranteed uniqueness
- **Cons**: No deduplication, no integrity verification, larger storage requirements

### Sequential Numbering

- **Pros**: Simple, compact
- **Cons**: No deduplication, requires coordination, security issues

### MD5 Hashing

- **Pros**: Faster computation, smaller hashes
- **Cons**: Known collision vulnerabilities, not suitable for security-sensitive applications

### Blake3 Hashing

- **Pros**: Faster than SHA-256, modern design
- **Cons**: Less ecosystem support, overkill for our use case

## Consequences

### Positive

- **Zero Duplicate Storage**: Identical files stored only once
- **Built-in Verification**: Hash mismatches immediately indicate corruption
- **Secure Paths**: Impossible to construct malicious file paths
- **Predictable Performance**: Even distribution across filesystem
- **Simple Cleanup**: Easy to identify and remove orphaned files

### Negative

- **Hash Computation**: CPU overhead for large files during upload
- **Fixed Structure**: Cannot easily reorganize files later
- **Debugging Complexity**: Hash-based names are not human-readable
- **Hash Collisions**: Theoretical risk (extremely unlikely with SHA-256)

## Implementation Details

### File Upload Process

1. Stream file content while computing SHA-256 hash
2. Store in temporary location during upload
3. Verify hash and move to final content-addressed location
4. Update database with hash and metadata

### Path Construction

```rust
fn build_content_path(hash: &str) -> PathBuf {
    let prefix1 = &hash[0..2];
    let prefix2 = &hash[2..4];
    let prefix3 = &hash[4..6];
    PathBuf::from(format!("{}/{}/{}/{}", prefix1, prefix2, prefix3, hash))
}
```

### Variant Storage

Processed variants (thumbnails, different formats) stored alongside originals:

- `abcdef123...` (original)
- `abcdef123...webp` (WebP variant)
- `abcdef123...avif` (AVIF variant)
- `abcdef123...thumb.webp` (thumbnail)

### Integrity Verification

- Periodic background jobs to verify file hashes
- Automatic re-processing if corruption detected
- Database stores both filename hash and content hash for verification
