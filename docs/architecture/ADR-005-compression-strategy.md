# ADR-005: Multi-Format Media Compression Strategy

## Status

Accepted

## Context

We need to optimize media delivery for web applications while maintaining quality and supporting different devices and
browsers. Modern image formats offer significant compression improvements over traditional formats, but browser support
varies.

## Decision

We will implement a multi-format compression strategy with AVIF as primary, WebP as fallback, and automatic thumbnail generation.

## Rationale

### Format Strategy

- **AVIF (Primary)**: Next-generation format with superior compression (50% smaller than JPEG)
- **WebP (Fallback)**: Widely supported modern format (25-34% smaller than JPEG)
- **Original Preservation**: Always keep original for re-processing with future algorithms

### Browser Support Analysis (2025)

- **AVIF**: ~90-93% browser support (Chrome, Firefox, recent Safari)
- **WebP**: ~96% browser support (universal modern browser support)
- **Graceful Degradation**: Serve best format supported by client

### Compression Benefits

- **Bandwidth Reduction**: Significant savings in data transfer costs
- **Performance**: Faster page loads and better user experience
- **Storage Efficiency**: Reduced storage requirements for processed variants
- **Mobile Optimization**: Especially important for mobile users

## Alternatives Considered

### Single Format Strategy

- **Pros**: Simpler implementation
- **Cons**: Misses optimization opportunities, poor future-proofing

### JPEG-XL

- **Pros**: Excellent compression, royalty-free
- **Cons**: Very limited browser support as of 2025

### Serve Original Only

- **Pros**: No processing overhead
- **Cons**: Poor performance, high bandwidth costs

## Implementation Strategy

### Automatic Processing Pipeline

1. **Upload**: Store original file with content-addressable naming
2. **Analysis**: Detect file type, dimensions, and characteristics
3. **Processing**: Generate optimized variants based on content type
4. **Storage**: Store variants alongside original with format suffixes

### Image Processing Rules

```text
Original → Multiple Variants:
├── .avif (primary, 85% quality)
├── .webp (fallback, 85% quality)
├── .thumb.webp (thumbnail, 256px max dimension)
└── .preview.webp (preview, 1024px max dimension)
```

### Video Processing Rules

```text
Original → Variants:
├── .thumb.webp (poster frame thumbnail)
├── .preview.mp4 (compressed preview clip)
└── .720p.mp4 (720p resolution variant)
```

### Quality Settings

- **AVIF**: 85% quality for optimal size/quality balance
- **WebP**: 85% quality to match AVIF visual quality
- **Thumbnails**: 80% quality (acceptable for small sizes)
- **Adaptive Quality**: Lower quality for very large images

## Consequences

### Positive

- **Significant Bandwidth Savings**: 30-50% reduction in data transfer
- **Improved Performance**: Faster loading times, better user experience
- **Future-Proof**: Easy to add new formats as browser support improves
- **Flexible Serving**: Can serve optimal format per client capability
- **Cost Savings**: Reduced storage and bandwidth costs

### Negative

- **Processing Overhead**: CPU and time cost for variant generation
- **Storage Multiplication**: Multiple variants increase storage usage
- **Complexity**: More complex serving logic and cache management
- **Initial Processing Delay**: Time required to generate variants

## Implementation Details

### Content Negotiation

Use HTTP Accept headers and User-Agent detection to serve optimal format:

```text
Accept: image/avif,image/webp,image/*
→ Serve AVIF if available

Accept: image/webp,image/*
→ Serve WebP if available

Accept: image/*
→ Serve original or WebP
```

### Lazy Generation

- Generate variants on-demand for first request
- Cache generation status in database
- Background job queue for proactive processing

### Library Choices

- **image-rs**: Rust-native image processing library
- **ez-ffmpeg**: Safe wrapper around FFmpeg for video processing
- **Performance**: Async processing to avoid blocking requests

### URL Structure

```text
/media/{hash}           # Serves best format for client
/media/{hash}.avif      # Specific format request
/media/{hash}.webp      # Specific format request
/media/{hash}.thumb     # Thumbnail (best format)
```

### Quality Monitoring

- Track compression ratios and file sizes
- Monitor visual quality through automated testing
- A/B testing for optimal quality settings
